use crate::{
    errors::AppError,
    models::{AppErrorPayload, DictationMode, DictationSnapshot, DictationState},
};

#[derive(Debug, Default)]
pub struct DictationStateMachine {
    snapshot: DictationSnapshot,
}

impl DictationStateMachine {
    pub fn snapshot(&self) -> DictationSnapshot {
        self.snapshot.clone()
    }

    pub fn begin(&mut self, session_id: String, mode: DictationMode) -> Result<(), AppError> {
        if self.snapshot.state != DictationState::Idle {
            return Err(AppError::SessionAlreadyActive);
        }
        self.snapshot.session_id = Some(session_id);
        self.snapshot.mode = Some(mode);
        self.snapshot.started_at = Some(chrono::Utc::now());
        self.snapshot.interim_transcript.clear();
        self.snapshot.error = None;
        self.transition(DictationState::Starting)
    }

    pub fn transition(&mut self, next: DictationState) -> Result<(), AppError> {
        if !allowed(self.snapshot.state, next) {
            return Err(AppError::InvalidTransition(format!(
                "{:?} -> {:?}",
                self.snapshot.state, next
            )));
        }
        log::debug!(
            "dictation state transition: {:?} -> {:?}",
            self.snapshot.state,
            next
        );
        self.snapshot.state = next;
        Ok(())
    }

    pub fn set_interim(&mut self, text: String) {
        self.snapshot.interim_transcript = text;
    }

    pub fn set_mode(&mut self, mode: DictationMode) {
        self.snapshot.mode = Some(mode);
    }

    pub fn fail(&mut self, error: AppErrorPayload) {
        self.snapshot.state = DictationState::Error;
        self.snapshot.error = Some(error);
    }

    pub fn reset(&mut self) {
        self.snapshot = DictationSnapshot::default();
    }
}

fn allowed(from: DictationState, to: DictationState) -> bool {
    use DictationState::*;
    matches!(
        (from, to),
        (Idle, Starting)
            | (
                Starting,
                ListeningPushToTalk | ListeningHandsFree | Error | Cancelled
            )
            | (
                ListeningPushToTalk | ListeningHandsFree,
                FinalizingAudio | Cancelled | Error
            )
            | (ListeningPushToTalk, ListeningHandsFree)
            | (FinalizingAudio, Transcribing | Cancelled | Error)
            | (Transcribing, Cleaning | Error | Cancelled)
            | (Cleaning, Inserting | Error | Cancelled)
            | (Inserting, Completed | Error)
            | (Completed | Cancelled | Error, Idle)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DictationState::*;

    #[test]
    fn rejects_overlapping_sessions() {
        let mut machine = DictationStateMachine::default();
        machine
            .begin("one".into(), DictationMode::PushToTalk)
            .unwrap();
        assert!(matches!(
            machine.begin("two".into(), DictationMode::PushToTalk),
            Err(AppError::SessionAlreadyActive)
        ));
    }

    #[test]
    fn accepts_the_push_to_talk_pipeline() {
        let mut machine = DictationStateMachine::default();
        machine
            .begin("one".into(), DictationMode::PushToTalk)
            .unwrap();
        for state in [
            ListeningPushToTalk,
            FinalizingAudio,
            Transcribing,
            Cleaning,
            Inserting,
            Completed,
            Idle,
        ] {
            machine.transition(state).unwrap();
        }
        assert_eq!(machine.snapshot().state, Idle);
    }

    #[test]
    fn promotes_push_to_talk_to_hands_free() {
        let mut machine = DictationStateMachine::default();
        machine
            .begin("one".into(), DictationMode::PushToTalk)
            .unwrap();
        machine.transition(ListeningPushToTalk).unwrap();
        machine.set_mode(DictationMode::HandsFree);
        machine.transition(ListeningHandsFree).unwrap();
        assert_eq!(machine.snapshot().mode, Some(DictationMode::HandsFree));
    }
}
