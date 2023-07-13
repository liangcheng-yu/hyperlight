#![allow(dead_code)]
use serde_json::{json, to_string_pretty, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tracing::Subscriber;
use tracing_core::{
    event::Event,
    metadata::Metadata,
    span::{Attributes, Current, Id, Record},
    Level, LevelFilter,
};
use tracing_serde::AsSerde;

#[derive(Debug, Clone)]
pub struct TracingSubscriber {
    span_metadata: Arc<Mutex<HashMap<u64, &'static Metadata<'static>>>>,
    spans: Arc<Mutex<HashMap<u64, Value>>>,
    events: Arc<Mutex<Vec<Value>>>,
    level_filter: LevelFilter,
    next_id: Arc<AtomicUsize>,
    span_stack: Arc<Mutex<Vec<Id>>>,
}

impl TracingSubscriber {
    pub(crate) fn new(trace_level: Level) -> Self {
        Self {
            spans: Arc::new(Mutex::new(HashMap::new())),
            span_metadata: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            level_filter: trace_level.into(),
            next_id: Arc::new(AtomicUsize::new(1)),
            span_stack: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn get_spans(&self) -> HashMap<u64, Value> {
        self.spans.lock().expect("Failed to lock spans").clone()
    }

    pub(crate) fn get_span_metadata(&self, id: u64) -> &'static Metadata<'static> {
        self.span_metadata
            .lock()
            .expect("Failed to lock span metadata")
            .get(&id)
            .unwrap_or_else(|| panic!("Failed to get span metadata ID {}", id))
    }

    pub(crate) fn get_span(&self, id: u64) -> Value {
        match self.spans.lock().expect("Failed to lock spans").get(&id) {
            Some(span) => span.clone(),
            None => panic!("No span found with id {}", id),
        }
    }

    pub(crate) fn get_events(&self) -> Vec<Value> {
        self.events.lock().expect("Failed to lock events").clone()
    }

    pub(crate) fn test_trace_records<F: Fn(&HashMap<u64, Value>, &Vec<Value>)>(&self, f: F) {
        f(&self.get_spans(), &self.get_events());
        self.events.lock().expect("Failed to lock events").clear();
    }

    pub(crate) fn clear(&self) {
        self.spans.lock().expect("Failed to lock spans").clear();
        self.events.lock().expect("Failed to lock events").clear();
        self.span_stack
            .lock()
            .expect("Failed to lock span stack")
            .clear();
        self.span_metadata
            .lock()
            .expect("Failed to span metadata")
            .clear();
        self.next_id.store(1, Ordering::Relaxed);
    }
}

impl Subscriber for TracingSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= &self.level_filter
    }

    fn new_span(&self, span_attributes: &Attributes<'_>) -> Id {
        let span_id = self.next_id.fetch_add(1, Ordering::Relaxed) as u64;
        let id = Id::from_u64(span_id);
        let json = json!({
        "span": {
            "id": id.as_serde(),
            "attributes": span_attributes.as_serde(),

        }});
        println!(
            "Thread {:?} {}",
            std::thread::current().id(),
            to_string_pretty(&json).expect("Failed to pretty print json")
        );
        self.spans
            .lock()
            .expect("Failed to lock spans")
            .insert(span_id, json);
        let metadata = span_attributes.metadata();
        self.span_metadata
            .lock()
            .expect("Failed to lock span metadata")
            .insert(span_id, metadata);
        id
    }

    fn record(&self, id: &Id, values: &Record<'_>) {
        let span_id = id.into_u64();
        let mut map = self.spans.lock().expect("Failed to lock spans");
        let entry = &mut *map
            .get_mut(&span_id)
            .unwrap_or_else(|| panic!("Failed to get span with ID {}", id.into_u64()));
        let json_object = entry
            .as_object_mut()
            .unwrap_or_else(|| panic!("Span entry is not an object {}", id.into_u64()));
        let mut json_values = json!(values.as_serde());
        println!(
            "Thread {:?} span {} values: {}",
            std::thread::current().id(),
            &span_id,
            to_string_pretty(&json_values).expect("Failed to pretty print json")
        );
        let json_values = json_values
            .as_object_mut()
            .expect("Record is not an object");
        json_object
            .get_mut("span")
            .expect("span not found in json")
            .as_object_mut()
            .expect("span was not an object")
            .get_mut("attributes")
            .expect("attributes not found in json")
            .as_object_mut()
            .expect("attributes was not an object")
            .append(json_values);
        println!(
            "Thread {:?} Updated Span {} values: {}",
            std::thread::current().id(),
            &span_id,
            to_string_pretty(&json_object).expect("Failed to pretty print json")
        );
    }

    fn event(&self, event: &Event<'_>) {
        let json = json!({
            "event": event.as_serde(),
        });
        println!(
            "Thread {:?} {}",
            std::thread::current().id(),
            to_string_pretty(&json).expect("Failed to pretty print json")
        );
        self.events
            .lock()
            .expect("Failed to lock events")
            .push(json);
    }

    fn current_span(&self) -> Current {
        if self
            .span_stack
            .lock()
            .expect("Failed to lock span stack")
            .is_empty()
        {
            return Current::none();
        }

        let stack = self.span_stack.lock().expect("Failed to lock span stack");
        let id = stack.last().expect("Failed to get last span from stack");
        let map = self
            .span_metadata
            .lock()
            .expect("Failed to lock span metadata");
        let metadata = *map
            .get(&id.into_u64())
            .unwrap_or_else(|| panic!("Failed to get span metadata ID {}", id.into_u64()));
        Current::new(id.clone(), metadata)
    }

    fn enter(&self, span: &Id) {
        println!(
            "Thread {:?} Entered Span {}",
            std::thread::current().id(),
            span.into_u64()
        );
        let mut stack = self.span_stack.lock().expect("Failed to lock span stack");
        stack.push(span.clone());
    }

    fn exit(&self, span: &Id) {
        println!(
            "Thread {:?} Exited Span {}",
            std::thread::current().id(),
            span.into_u64()
        );
        let mut stack = self.span_stack.lock().expect("Failed to lock span stack");
        _ = stack.pop();
    }

    // We are not interested in this method for testing

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
}
