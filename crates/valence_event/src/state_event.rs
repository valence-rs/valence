use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::iter::Chain;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::slice::Iter;

use bevy_ecs::prelude::Event;
use bevy_ecs::system::{Local, Res, ResMut, Resource, SystemParam};

pub struct EventId<E: Event> {
    /// Uniquely identifies the event associated with this ID.
    // This value corresponds to the order in which each event was added to the world.
    pub id: usize,
    _marker: PhantomData<E>,
}

impl<E: Event> Copy for EventId<E> {}
impl<E: Event> Clone for EventId<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E: Event> fmt::Display for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<E: Event> fmt::Debug for EventId<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "event<{}>#{}",
            std::any::type_name::<E>().split("::").last().unwrap(),
            self.id,
        )
    }
}

impl<E: Event> Eq for EventId<E> {}
impl<E: Event> PartialEq for EventId<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<E: Event> PartialOrd for EventId<E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<E: Event> Ord for EventId<E> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

/// A unique identifier for a state usable by multiple events types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId(usize);

/// Defines a struct that can be used as a state for an event.
pub trait State: Send + Sync + 'static {
    fn get(&self) -> bool;
}

/// Resource that holds all states for all events.
#[derive(Resource)]
pub struct States {
    state_count: usize,
    states: HashMap<StateId, Box<dyn State>>,
}

impl Default for States {
    fn default() -> Self {
        Self {
            state_count: 0,
            states: Default::default(),
        }
    }
}

impl States {
    /// Adds a new state to the state registry.
    pub fn add(&mut self, state: impl State) -> StateId {
        let id = StateId(self.state_count);
        self.state_count += 1;
        self.states.insert(id, Box::new(state));
        id
    }

    /// Gets a state by id.
    pub fn get(&self, id: StateId) -> &dyn State {
        self.states.get(&id).unwrap().as_ref()
    }

    /// Gets a mutable state by id.
    pub fn get_mut(&mut self, id: StateId) -> &mut dyn State {
        self.states.get_mut(&id).unwrap().as_mut()
    }
}

/// An event linked to a specific state.
#[derive(Debug)]
struct EventWithStateInstance<E: Event> {
    pub event_id: EventId<E>,
    pub event: E,
    pub state_id: StateId,
}

///
#[derive(Debug)]
struct EventWithStateSequence<E: Event> {
    events: Vec<EventWithStateInstance<E>>,
    start_event_count: usize,
}

// Derived Default impl would incorrectly require E: Default
impl<E: Event> Default for EventWithStateSequence<E> {
    fn default() -> Self {
        Self {
            events: Default::default(),
            start_event_count: Default::default(),
        }
    }
}

impl<E: Event> Deref for EventWithStateSequence<E> {
    type Target = Vec<EventWithStateInstance<E>>;

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl<E: Event> DerefMut for EventWithStateSequence<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.events
    }
}

/// A resource that holds all events of a specific type in a double buffer.
#[derive(Debug, Resource)]
pub struct EventsWithState<E: Event> {
    /// Holds the oldest still active events.
    /// Note that a.start_event_count + a.len() should always ===
    /// events_b.start_event_count.
    events_a: EventWithStateSequence<E>,
    /// Holds the newer events.
    events_b: EventWithStateSequence<E>,
    event_count: usize,
}

// Derived Default impl would incorrectly require E: Default
impl<E: Event> Default for EventsWithState<E> {
    fn default() -> Self {
        Self {
            events_a: Default::default(),
            events_b: Default::default(),
            event_count: Default::default(),
        }
    }
}

impl<E: Event> EventsWithState<E> {
    /// Returns the index of the oldest event stored in the event buffer.
    pub fn oldest_event_count(&self) -> usize {
        self.events_a
            .start_event_count
            .min(self.events_b.start_event_count)
    }

    pub fn send(&mut self, event: E, state_id: StateId) {
        let event_id = EventId {
            id: self.event_count,
            _marker: PhantomData,
        };
        // detailed_trace!("Events::send() -> id: {}", event_id);

        let event_instance = EventWithStateInstance {
            event_id,
            event,
            state_id,
        };

        self.events_b.push(event_instance);
        self.event_count += 1;
    }

    /// Gets a new [`ManualStateEventReader`]. This will include all events
    /// already in the event buffers.
    pub fn get_reader(&self) -> ManualEventWithStateReader<E> {
        ManualEventWithStateReader::default()
    }

    /// Gets a new [`ManualStateEventReader`]. This will ignore all events
    /// already in the event buffers. It will read all future events.
    pub fn get_reader_current(&self) -> ManualEventWithStateReader<E> {
        ManualEventWithStateReader {
            last_event_count: self.event_count,
            ..Default::default()
        }
    }

    /// Swaps the event buffers and clears the oldest event buffer. In general,
    /// this should be called once per frame/update.
    pub fn update(&mut self) {
        std::mem::swap(&mut self.events_a, &mut self.events_b);
        self.events_b.clear();
        self.events_b.start_event_count = self.event_count;
        debug_assert_eq!(
            self.events_a.start_event_count + self.events_a.len(),
            self.events_b.start_event_count
        );
    }

    /// A system that calls [`Events::update`] once per frame.
    pub fn update_system(mut events: ResMut<Self>) {
        events.update();
    }

    #[inline]
    fn reset_start_event_count(&mut self) {
        self.events_a.start_event_count = self.event_count;
        self.events_b.start_event_count = self.event_count;
    }

    /// Removes all events.
    #[inline]
    pub fn clear(&mut self) {
        self.reset_start_event_count();
        self.events_a.clear();
        self.events_b.clear();
    }

    /// Returns the number of events currently stored in the event buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.events_a.len() + self.events_b.len()
    }

    /// Returns true if there are no events currently stored in the event
    /// buffer.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Creates a draining iterator that removes all events.
    pub fn drain(&mut self) -> impl Iterator<Item = E> + '_ {
        self.reset_start_event_count();

        // Drain the oldest events first, then the newest
        self.events_a
            .drain(..)
            .chain(self.events_b.drain(..))
            .map(|i| i.event)
    }

    /// Iterates over events that happened since the last "update" call.
    /// WARNING: You probably don't want to use this call. In most cases you
    /// should use an [`EventWithStateReader`]. You should only use this if
    /// you know you only need to consume events between the last `update()`
    /// call and your call to `iter_current_update_events`. If events happen
    /// outside that window, they will not be handled. For example, any events
    /// that happen after this call and before the next `update()` call will
    /// be dropped.
    pub fn iter_current_update_events(&self) -> impl ExactSizeIterator<Item = &E> {
        self.events_b.iter().map(|i| &i.event)
    }

    /// Get a specific event by id if it still exists in the events buffer.
    pub fn get_event(&self, id: usize) -> Option<(&E, EventId<E>)> {
        if id < self.oldest_id() {
            return None;
        }

        let sequence = self.sequence(id);
        let index = id.saturating_sub(sequence.start_event_count);

        sequence
            .get(index)
            .map(|instance| (&instance.event, instance.event_id))
    }

    /// Oldest id still in the events buffer.
    pub fn oldest_id(&self) -> usize {
        self.events_a.start_event_count
    }

    /// Which event buffer is this event id a part of.
    fn sequence(&self, id: usize) -> &EventWithStateSequence<E> {
        if id < self.events_b.start_event_count {
            &self.events_a
        } else {
            &self.events_b
        }
    }
}

#[derive(SystemParam)]
pub struct EventWithStateWriter<'w, E: Event> {
    events: ResMut<'w, EventsWithState<E>>,
    states: ResMut<'w, States>,
}

impl<'w, E: Event> EventWithStateWriter<'w, E> {
    pub fn send(&mut self, event: E, state: impl State) {
        let state_id = self.states.add(state);
        self.events.send(event, state_id);
    }

    pub fn send_with_state(&mut self, event: E, state_id: StateId) {
        self.events.send(event, state_id);
    }
}

///
#[derive(SystemParam)]
pub struct EventWithStateReader<'s, 'w, E: Event> {
    reader: Local<'s, ManualEventWithStateReader<E>>,
    events: Res<'w, EventsWithState<E>>,
    states: Res<'w, States>,
}

impl<'s, 'w, E: Event> EventWithStateReader<'s, 'w, E> {
    pub fn iter(&mut self) -> ManualEventWithStateIterator<'_, E> {
        self.reader.iter(&self.events, &self.states)
    }
}

impl<'a, E: Event> IntoIterator for &'a mut EventWithStateReader<'_, '_, E> {
    type Item = (&'a E, &'a dyn State);
    type IntoIter = ManualEventWithStateIterator<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug)]
pub struct ManualEventWithStateReader<E: Event> {
    last_event_count: usize,
    _marker: PhantomData<E>,
}

impl<E: Event> Default for ManualEventWithStateReader<E> {
    fn default() -> Self {
        ManualEventWithStateReader {
            last_event_count: 0,
            _marker: Default::default(),
        }
    }
}

impl<E: Event> ManualEventWithStateReader<E> {
    pub fn iter<'a>(
        &'a mut self,
        events: &'a EventsWithState<E>,
        states: &'a States,
    ) -> ManualEventWithStateIterator<'a, E> {
        ManualEventWithStateReader::new(self, events, states)
    }
}

#[derive()]
pub struct ManualEventWithStateIterator<'a, E: Event> {
    reader: &'a mut ManualEventWithStateReader<E>,
    chain: Chain<Iter<'a, EventWithStateInstance<E>>, Iter<'a, EventWithStateInstance<E>>>,
    states: &'a States,
    unread: usize,
}

impl<'a, E: Event> ManualEventWithStateReader<E> {
    pub fn new(
        reader: &'a mut ManualEventWithStateReader<E>,
        events: &'a EventsWithState<E>,
        states: &'a States,
    ) -> ManualEventWithStateIterator<'a, E> {
        let a_index = (reader.last_event_count).saturating_sub(events.events_a.start_event_count);
        let b_index = (reader.last_event_count).saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        // Ensure `len` is implemented correctly
        // debug_assert_eq!(unread_count, reader.len(events));
        reader.last_event_count = events.event_count - unread_count;
        // Iterate the oldest first, then the newer events
        let chain = a.iter().chain(b.iter());

        ManualEventWithStateIterator {
            reader,
            chain,
            states,
            unread: unread_count,
        }
    }
}

impl<'a, E: Event> Iterator for ManualEventWithStateIterator<'a, E> {
    type Item = (&'a E, &'a dyn State);

    fn next(&mut self) -> Option<Self::Item> {
        match self
            .chain
            .next()
            .map(|instance| (&instance.event, instance.state_id, instance.event_id))
        {
            Some(item) => {
                // detailed_trace!("EventWithStateReader::iter() -> {}", item.1);
                self.reader.last_event_count += 1;
                self.unread -= 1;
                Some((item.0, self.states.get(item.1)))
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.unread, Some(self.unread))
    }
}

#[derive(SystemParam)]
pub struct EventWithStateMutReader<'s, 'w, E: Event> {
    reader: Local<'s, ManualEventWithStateReader<E>>,
    events: Res<'w, EventsWithState<E>>,
    states: ResMut<'w, States>,
}

impl<'s, 'w, E: Event> EventWithStateMutReader<'s, 'w, E> {
    pub fn iter_mut(&mut self) -> ManualEventWithStateMutIterator<'_, E> {
        self.reader.iter_mut(&self.events, &mut self.states)
    }
}

impl<'a, E: Event> IntoIterator for &'a mut EventWithStateMutReader<'_, '_, E> {
    type Item = (&'a E, &'a mut dyn State);
    type IntoIter = ManualEventWithStateMutIterator<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive()]
pub struct ManualEventWithStateMutIterator<'a, E: Event> {
    reader: &'a mut ManualEventWithStateReader<E>,
    chain: Chain<Iter<'a, EventWithStateInstance<E>>, Iter<'a, EventWithStateInstance<E>>>,
    states: &'a mut States,
    unread: usize,
}

impl<E: Event> ManualEventWithStateReader<E> {
    pub fn iter_mut<'a>(
        &'a mut self,
        events: &'a EventsWithState<E>,
        states: &'a mut States,
    ) -> ManualEventWithStateMutIterator<'a, E> {
        ManualEventWithStateMutIterator::new(self, events, states)
    }
}

impl<'a, E: Event> ManualEventWithStateMutIterator<'a, E> {
    pub fn new(
        reader: &'a mut ManualEventWithStateReader<E>,
        events: &'a EventsWithState<E>,
        states: &'a mut States,
    ) -> ManualEventWithStateMutIterator<'a, E> {
        let a_index = (reader.last_event_count).saturating_sub(events.events_a.start_event_count);
        let b_index = (reader.last_event_count).saturating_sub(events.events_b.start_event_count);
        let a = events.events_a.get(a_index..).unwrap_or_default();
        let b = events.events_b.get(b_index..).unwrap_or_default();

        let unread_count = a.len() + b.len();
        // Ensure `len` is implemented correctly
        // debug_assert_eq!(unread_count, reader.len(events));
        reader.last_event_count = events.event_count - unread_count;
        // Iterate the oldest first, then the newer events
        let chain = a.iter().chain(b.iter());

        ManualEventWithStateMutIterator {
            reader,
            chain,
            states,
            unread: unread_count,
        }
    }
}

impl<'a, E: Event> Iterator for ManualEventWithStateMutIterator<'a, E> {
    type Item = (&'a E, &'a mut dyn State);

    fn next(&mut self) -> Option<Self::Item> {
        match self
            .chain
            .next()
            .map(|instance| (&instance.event, instance.state_id))
        {
            Some(item) => {
                self.reader.last_event_count += 1;
                self.unread -= 1;
                Some((item.0, self.states.get_mut(item.1)))
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.unread, Some(self.unread))
    }
}
