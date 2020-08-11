use std::env;
use std::fs::{File, OpenOptions};
use std::io::Write;
use weasel::battle::BattleRules;
use weasel::event::{ClientSink, EventSink, EventSinkId, VersionedEventWrapper};
use weasel::serde::FlatVersionedEvent;
use weasel::WeaselResult;

/// A sink that dumps events into a file.
pub struct AutosaveSink<R: BattleRules> {
    id: EventSinkId,
    file: File,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: BattleRules + 'static> AutosaveSink<R> {
    pub fn new(id: EventSinkId, filename: &str) -> Self {
        // Open the autosave file.
        let mut path = env::temp_dir();
        path.push(filename);
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .unwrap();
        Self {
            id,
            file,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<R: BattleRules> EventSink for AutosaveSink<R> {
    fn id(&self) -> EventSinkId {
        self.id
    }

    fn on_disconnect(&mut self) {
        println!("oh no! the sink got disconnected!")
    }
}

impl<R: BattleRules + 'static> ClientSink<R> for AutosaveSink<R> {
    fn send(&mut self, event: &VersionedEventWrapper<R>) -> WeaselResult<(), R> {
        // Serialize the event to json.
        let flat_event: FlatVersionedEvent<R> = event.clone().into();
        let json = serde_json::to_string(&flat_event).unwrap();
        // Append to the file.
        self.file.write_all(json.as_bytes()).unwrap();
        // Append a delimiter between json objects.
        self.file.write_all(b"#").unwrap();
        Ok(())
    }
}
