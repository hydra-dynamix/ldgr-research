use std::env;
use std::path::PathBuf;

use ldgr::telemetry::buffer::LocalSequenceBuffer;
use ldgr::telemetry::transition::{
    StateCode, CANCELLED, COMPLETED_INCONCLUSIVE, COMPLETED_NEGATIVE, COMPLETED_POSITIVE,
    RESEARCH_WORKFLOW_V1, RUNNING,
};

use crate::schema::ExperimentStatus;

pub(crate) fn best_effort_emit_experiment_status_transition(
    previous: &str,
    next: ExperimentStatus,
) {
    let _ = emit_experiment_status_transition(previous, next);
}

fn emit_experiment_status_transition(previous: &str, next: ExperimentStatus) -> anyhow::Result<()> {
    let Some(terminal) = terminal_for_experiment_status(next) else {
        return Ok(());
    };
    let Some(ldgr_home) = telemetry_ldgr_home() else {
        return Ok(());
    };
    let Some(mut buffer) =
        LocalSequenceBuffer::begin_after_commit(ldgr_home, &RESEARCH_WORKFLOW_V1)?
    else {
        return Ok(());
    };

    if previous == "running" || (previous == "planned" && terminal != CANCELLED) {
        buffer.submit_committed(RUNNING)?;
    }
    buffer.submit_committed(terminal)?;
    Ok(())
}

fn terminal_for_experiment_status(status: ExperimentStatus) -> Option<StateCode> {
    match status {
        ExperimentStatus::Completed => Some(COMPLETED_POSITIVE),
        // A failed research experiment is an executed negative finding or
        // counterexample. Machinery failures must use the operational-failure
        // terminal, not this experiment outcome.
        ExperimentStatus::Failed => Some(COMPLETED_NEGATIVE),
        ExperimentStatus::Inconclusive => Some(COMPLETED_INCONCLUSIVE),
        ExperimentStatus::Superseded => Some(CANCELLED),
        ExperimentStatus::Planned | ExperimentStatus::Running => None,
    }
}

fn telemetry_ldgr_home() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(".ldgr"))
}

#[cfg(test)]
mod tests {
    use ldgr::telemetry::adapter_conformance::{
        verify_adapter_telemetry_conformance, TerminalPath,
    };
    use ldgr::telemetry::transition::{
        CommittedSequence, NormalizedTerminal, TransitionAcceptance, OPERATIONAL_FAILURE, PENDING,
    };

    use super::*;

    const POSITIVE_PATH: &[StateCode] = &[PENDING, RUNNING, COMPLETED_POSITIVE];
    const NEGATIVE_PATH: &[StateCode] = &[PENDING, RUNNING, COMPLETED_NEGATIVE];
    const INCONCLUSIVE_PATH: &[StateCode] = &[PENDING, RUNNING, COMPLETED_INCONCLUSIVE];
    const OPERATIONAL_FAILURE_PATH: &[StateCode] = &[PENDING, OPERATIONAL_FAILURE];
    const CANCELLED_PATH: &[StateCode] = &[PENDING, CANCELLED];

    #[test]
    fn research_workflow_protocol_preserves_failed_experiment_as_completed_negative(
    ) -> anyhow::Result<()> {
        let mut sequence = CommittedSequence::begin_after_commit(&RESEARCH_WORKFLOW_V1)?;
        assert_eq!(
            sequence.submit_committed(RUNNING)?,
            TransitionAcceptance::Intermediate
        );
        assert_eq!(
            sequence.submit_committed(COMPLETED_NEGATIVE)?,
            TransitionAcceptance::Terminal(NormalizedTerminal::CompletedNegative)
        );
        assert_eq!(sequence.numerical_states(), NEGATIVE_PATH);
        Ok(())
    }

    #[test]
    fn research_workflow_protocol_conforms_to_core_adapter_contract() -> anyhow::Result<()> {
        let report = verify_adapter_telemetry_conformance(
            &RESEARCH_WORKFLOW_V1,
            &[
                TerminalPath::new(NormalizedTerminal::CompletedPositive, POSITIVE_PATH),
                TerminalPath::new(NormalizedTerminal::CompletedNegative, NEGATIVE_PATH),
                TerminalPath::new(NormalizedTerminal::CompletedInconclusive, INCONCLUSIVE_PATH),
                TerminalPath::new(
                    NormalizedTerminal::OperationalFailure,
                    OPERATIONAL_FAILURE_PATH,
                ),
                TerminalPath::new(NormalizedTerminal::Cancelled, CANCELLED_PATH),
            ],
        )?;
        assert_eq!(report.endpoint, "/sequences/research-workflow/v1");
        assert!(report.terminal_payloads.iter().any(|payload| {
            payload.terminal == NormalizedTerminal::CompletedNegative
                && payload.payload == b"[0,1,4]"
        }));
        Ok(())
    }
}
