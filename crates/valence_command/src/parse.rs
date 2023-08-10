use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;

use bevy_ecs::system::{ReadOnlySystemParam, SystemParamItem, SystemState};
use bevy_ecs::world::World;
use valence_core::text::Text;

use crate::command::CommandExecutorBase;
use crate::nodes::NodeSuggestion;
use crate::pkt;
use crate::reader::{ArcStrReader, StrLocated, StrReader};
use crate::suggestions::Suggestion;

pub type ParseResult<T> = Result<T, StrLocated<Text>>;

/// Identifies any object that can be parsed in a command
#[async_trait::async_trait]
pub trait Parse: 'static {
    type Item<'a>: 'a + Send + Sized;

    /// Any data that can be possibly passed to [`Parse`] to change it's
    /// behaviour.
    type Data<'a>: 'a + Sync + Send;

    /// A type which is used to calculate suggestions after [`Parse::parse`] or
    /// [`Parse::skip`] methods were called.
    type Suggestions: 'static + Sync + Send + Default;

    /// A param which is used to calculate [`Parse::SuggestionsAsyncData`]    
    type SuggestionsParam: 'static + ReadOnlySystemParam;

    /// A data which will be then given to the async function
    /// [`Parse::suggestions`]
    type SuggestionsAsyncData: 'static + Send;

    const VANILLA: bool;

    fn parse_id() -> TypeId;

    fn item_id() -> TypeId {
        TypeId::of::<Self::Item<'static>>()
    }

    /// Parses value from a given string and moves reader to the place where the
    /// value is ended.
    /// ### May not
    /// - give different results on the same data and reader string
    fn parse<'a>(
        data: &Self::Data<'a>,
        suggestions: &mut Self::Suggestions,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self::Item<'a>>;

    /// Does the same as [`Parse::parse`] but doesn't return any value. Useful
    /// for values, which contain some stuff that needs heap allocation
    fn skip<'a>(
        data: &Self::Data<'a>,
        suggestions: &mut Self::Suggestions,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<()> {
        Self::parse(data, suggestions, reader).map(|_| ())
    }

    fn brigadier(data: &Self::Data<'_>) -> Option<pkt::Parser<'static>>;

    fn brigadier_suggestions(data: &Self::Data<'_>) -> Option<NodeSuggestion>;

    /// Creates a data which will be passed then to
    /// [`Parse::suggestions`] method
    fn create_suggestions_data(
        data: &Self::Data<'_>,
        command: ArcStrReader,
        executor: CommandExecutorBase,
        suggestions: &Self::Suggestions,
        param: SystemParamItem<Self::SuggestionsParam>,
    ) -> Self::SuggestionsAsyncData;

    async fn suggestions(
        command: ArcStrReader,
        executor: CommandExecutorBase,
        suggestions: Box<Self::Suggestions>,
        async_data: Self::SuggestionsAsyncData,
    ) -> StrLocated<Cow<'static, [Suggestion<'static>]>>;
}

#[derive(Clone, Debug)]
pub(crate) struct ParseResultsWrite(pub Vec<u64>);

// we don't want rust to change variable's places in the struct
#[repr(C)]
pub(crate) struct RawAny<I> {
    obj_drop: Option<unsafe fn(*mut I)>,
    tid: TypeId,
    obj: I,
}

impl<I> Drop for RawAny<I> {
    fn drop(&mut self) {
        if let Some(obj_drop) = self.obj_drop {
            // SAFETY: Guarantied by struct
            unsafe { obj_drop(&mut self.obj as *mut I) }
        }
    }
}

impl<I> RawAny<I> {
    pub fn new<T: Parse>(obj: I) -> Self {
        Self {
            obj_drop: if std::mem::needs_drop::<I>() {
                Some(std::ptr::drop_in_place)
            } else {
                None
            },
            tid: T::item_id(),
            obj,
        }
    }

    pub fn write(self, vec: &mut Vec<u64>) {
        let mut to_write = (0, self);

        // Depends on system. if less than 64bit then the size does not change otherwise
        // changes
        let len = std::mem::size_of::<(usize, RawAny<I>)>() / 8;

        to_write.0 = len;

        // TypeId is an u64, so it shouldn't panic
        debug_assert!(std::mem::size_of::<(usize, RawAny<I>)>() % 8 == 0);

        let self_ptr = &to_write as *const (usize, RawAny<I>) as *const u64;

        // SAFETY: self_ptr is the pointer to our struct, the size of u64 we can put
        // into the struct is equal to len variable
        unsafe {
            vec.extend_from_slice(std::slice::from_raw_parts(self_ptr, len));
        }

        // We wrote bytes to the vector and all objects there shouldn't be dropped so we
        // can use them in future
        std::mem::forget(to_write);
    }

    /// # Safety
    /// - Slice must be created using [`Self::write`] method
    pub unsafe fn read<'a, T: Parse>(slice: &mut &'a [u64]) -> &'a Self {
        if slice.len() <= std::mem::size_of::<(usize, RawAny<()>)>() / 8 {
            panic!("Slice doesn't contain essential information as length, drop and tid");
        }

        let slice_ptr = slice.as_ptr();

        // SAFETY: the caller
        let empty_any_ptr = unsafe { &*(slice_ptr as *const (usize, RawAny<()>)) };

        if empty_any_ptr.1.tid != T::item_id() {
            panic!("Tried to read the wrong object");
        }

        // SAFETY: we have checked if the type the caller has gave to us is the right
        // one.
        let right_any_ptr =
            unsafe { &*(empty_any_ptr as *const (usize, RawAny<()>) as *const (usize, RawAny<I>)) };

        *slice = &slice[right_any_ptr.0..];

        &right_any_ptr.1
    }

    /// # Safety
    /// - Vec must be filled using [`Self::write`] method
    pub unsafe fn drop_all(vec: &mut Vec<u64>) {
        let mut slice = vec.as_mut_slice();

        while !slice.is_empty() {
            let slice_ptr = slice.as_mut_ptr();

            let empty_any = slice_ptr as *mut (usize, RawAny<()>);

            // SAFETY: the pointer is right
            unsafe {
                std::ptr::drop_in_place(empty_any);
            }

            // SAFETY: the pointer is right
            slice = &mut slice[unsafe { (&*empty_any).0 }..];
        }
    }
}

impl ParseResultsWrite {
    pub fn write<'a, T: Parse>(&mut self, obj: T::Item<'a>) {
        RawAny::<T::Item<'a>>::new::<T>(obj).write(&mut self.0);
    }
}

impl Drop for ParseResultsWrite {
    fn drop(&mut self) {
        // SAFETY: only write method is public
        unsafe { RawAny::<()>::drop_all(&mut self.0) }
    }
}

#[derive(Clone, Debug)]
pub struct ParseResultsRead<'a>(&'a [u64]);

impl<'a> ParseResultsRead<'a> {
    pub fn read<T: Parse>(&mut self) -> &'a T::Item<'a> {
        // SAFETY: slice is from ParseResultsWrite
        unsafe { &RawAny::<T::Item<'a>>::read::<T>(&mut self.0).obj }
    }
}

#[derive(Debug)]
pub(crate) struct ParseResults {
    command: String,
    results: ParseResultsWrite,
}

impl ParseResults {
    pub fn new_empty(command: String) -> Self {
        Self {
            command,
            results: ParseResultsWrite(vec![]),
        }
    }

    pub fn to_write(&mut self) -> (StrReader, &mut ParseResultsWrite) {
        (StrReader::from_command(&self.command), &mut self.results)
    }

    pub fn to_read(&self) -> ParseResultsRead {
        ParseResultsRead(&self.results.0)
    }
}

pub(crate) trait ParseObject: Sync + Send {
    fn parse_id(&self) -> TypeId;

    fn initialize(&mut self, world: &mut World);

    fn obj_parse<'a>(
        &self,
        reader: &mut StrReader<'a>,
        fill: &mut ParseResultsWrite,
    ) -> (ParseResult<()>, Box<dyn Any>);

    fn obj_skip<'a>(&self, reader: &mut StrReader<'a>) -> (ParseResult<()>, Box<dyn Any>);

    fn obj_brigadier(&self) -> Option<pkt::Parser<'static>>;

    fn obj_brigadier_suggestions(&self) -> Option<NodeSuggestion>;

    fn obj_suggestions<'f>(
        &mut self,
        suggestions: Box<dyn Any>,
        command: ArcStrReader,
        executor: CommandExecutorBase,
        world: &World,
    ) -> Pin<Box<dyn Future<Output = StrLocated<Cow<'static, [Suggestion<'static>]>>> + Send + 'f>>;

    fn obj_apply_deferred(&mut self, world: &mut World);
}

pub(crate) struct ParseWithData<T: Parse> {
    pub data: T::Data<'static>,
    pub state: Option<SystemState<T::SuggestionsParam>>,
}

impl<T: Parse> ParseObject for ParseWithData<T> {
    fn parse_id(&self) -> TypeId {
        T::parse_id()
    }

    fn initialize(&mut self, world: &mut World) {
        if self.state.is_none() {
            self.state = Some(SystemState::new(world));
        }
    }

    fn obj_parse<'a>(
        &self,
        reader: &mut StrReader<'a>,
        fill: &mut ParseResultsWrite,
    ) -> (ParseResult<()>, Box<dyn Any>) {
        let mut suggestions = T::Suggestions::default();
        let result = T::parse(&self.data, &mut suggestions, unsafe {
            std::mem::transmute(reader)
        });
        (
            match result {
                Ok(obj) => {
                    fill.write::<T>(obj);
                    Ok(())
                }
                Err(e) => Err(e),
            },
            Box::new(suggestions),
        )
    }

    fn obj_skip<'a>(&self, reader: &mut StrReader<'a>) -> (ParseResult<()>, Box<dyn Any>) {
        let mut suggestions = T::Suggestions::default();
        let result = T::skip(&self.data, &mut suggestions, unsafe {
            std::mem::transmute(reader)
        });
        (result, Box::new(suggestions))
    }

    fn obj_brigadier(&self) -> Option<pkt::Parser<'static>> {
        T::brigadier(&self.data)
    }

    fn obj_brigadier_suggestions(&self) -> Option<NodeSuggestion> {
        T::brigadier_suggestions(&self.data)
    }

    fn obj_suggestions<'f>(
        &mut self,
        suggestion: Box<dyn Any>,
        command: ArcStrReader,
        executor: CommandExecutorBase,
        world: &World,
    ) -> Pin<Box<dyn Future<Output = StrLocated<Cow<'static, [Suggestion<'static>]>>> + Send + 'f>>
    {
        let suggestion: Box<T::Suggestions> = suggestion.downcast().unwrap();
        let param = self.state.as_mut().unwrap().get(world);
        let data =
            T::create_suggestions_data(&self.data, command.clone(), executor, &suggestion, param);
        T::suggestions(command, executor, suggestion, data)
    }

    fn obj_apply_deferred(&mut self, world: &mut World) {
        self.state.as_mut().unwrap().apply(world);
    }
}
