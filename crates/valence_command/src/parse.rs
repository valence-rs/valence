use std::any::TypeId;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;

use bevy_ecs::world::World;
use valence_core::text::Text;

use crate::command::{CommandExecutor, RealCommandExecutor};
use crate::pkt;
use crate::reader::{StrLocated, StrReader};
use crate::suggestions::{RawParseSuggestions, SuggestionAnswerer, SuggestionsTransaction};

pub type ParseResult<T> = Result<T, StrLocated<Text>>;

/// Identifies any object that can be parsed in a command
pub trait Parse<'a>: 'a + Send + Sized {
    /// Any data that can be possibly passed to [`Parse`] to change it's
    /// behaviour.
    type Data: 'a + Sync + Send;

    /// A type which is used to calculate suggestions after [`Parse::parse`] or
    /// [`Parse::skip`] methods were called.
    type Suggestions: 'a + Default;

    fn id() -> TypeId;

    /// Parses value from a given string and moves reader to the place where the
    /// value is ended.
    /// ### May not
    /// - give different results on the same data and reader string
    /// - panic if data is valid. Error should be passed as an [`Result::Err`]
    fn parse(
        data: &Self::Data,
        suggestions: &mut Self::Suggestions,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self>;

    /// Does the same as [`Parse::parse`] but doesn't return any value. Useful
    /// for values, which contain some stuff that needs heap allocation
    fn skip(
        data: &Self::Data,
        suggestions: &mut Self::Suggestions,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<()> {
        Self::parse(data, suggestions, reader).map(|_| ())
    }

    fn brigadier(data: &Self::Data) -> Option<pkt::Parser<'static>>;

    #[allow(unused_variables)]
    /// Returns true if this [`Parse`] is provided by vanilla minecraft
    fn vanilla(data: &Self::Data) -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ParseResultsWrite(pub Vec<u64>);

// we don't want rust to change variables places in the struct
#[repr(C)]
pub(crate) struct RawAny<T> {
    obj_drop: Option<unsafe fn(*mut T)>,
    tid: TypeId,
    obj: T,
}

impl<T> Drop for RawAny<T> {
    fn drop(&mut self) {
        if let Some(obj_drop) = self.obj_drop {
            // SAFETY: Guarantied by struct
            unsafe { obj_drop(&mut self.obj as *mut T) }
        }
    }
}

impl<T> RawAny<T> {
    pub fn new<'a>(obj: T) -> Self
    where
        T: Parse<'a>,
    {
        Self {
            obj_drop: if std::mem::needs_drop::<T>() {
                Some(std::ptr::drop_in_place)
            } else {
                None
            },
            tid: T::id(),
            obj,
        }
    }

    pub fn write(self, vec: &mut Vec<u64>) {
        let mut to_write = (0, self);

        // Depends on system. if less than 64bit then the size does not change otherwise
        // changes
        let len = std::mem::size_of::<(usize, RawAny<T>)>() / 8;

        to_write.0 = len;

        // TypeId is an u64, so it shouldn't panic
        debug_assert!(std::mem::size_of::<(usize, RawAny<T>)>() % 8 == 0);

        let self_ptr = &to_write as *const (usize, RawAny<T>) as *const u64;

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
    pub unsafe fn read<'a>(slice: &mut &'a [u64]) -> &'a Self
    where
        T: Parse<'a>,
    {
        if slice.len() <= std::mem::size_of::<(usize, RawAny<()>)>() / 8 {
            panic!("Slice doesn't contain essential information as length, drop and tid");
        }

        let slice_ptr = slice.as_ptr();

        // SAFETY: caller
        let empty_any_ptr = unsafe { &*(slice_ptr as *const (usize, RawAny<()>)) };

        if empty_any_ptr.1.tid != T::id() {
            panic!("Tried to read the wrong object");
        }

        // SAFETY: we have checked if the type caller has gave to us is the right one.
        let right_any_ptr =
            unsafe { &*(empty_any_ptr as *const (usize, RawAny<()>) as *const (usize, RawAny<T>)) };

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
    pub fn write<'a, T: Parse<'a>>(&mut self, obj: T) {
        RawAny::new(obj).write(&mut self.0);
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
    pub fn read<T: Parse<'a>>(&mut self) -> &'a T {
        // SAFETY: slice is from ParseResultsWrite
        unsafe { &RawAny::read(&mut self.0).obj }
    }
}

pub(crate) struct ParseResults {
    command: String,
    results: ParseResultsWrite,
}

impl ParseResults {

    /// # Safety
    /// Given [`ParseResultsWrite`] must be made from given command [`String`]
    pub unsafe fn new(command: String, results: ParseResultsWrite) -> Self {
        Self {
            command,
            results
        }
    }

    pub fn new_empty(command: String) -> Self {
        Self {
            command,
            results: ParseResultsWrite(vec![])
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
    /// Parses an object and writes it into the `fill` vec. Returns bytes which
    /// represents suggestions
    /// # Safety
    /// The implementation must write type id first into the `fill` vec and then
    /// the object itself. Returned pointer. Also the implementation must ensure
    /// that the parsed object has 'a lifetime
    unsafe fn obj_parse<'a>(
        &self,
        reader: &mut StrReader<'a>,
        fill: &mut ParseResultsWrite,
    ) -> (ParseResult<()>, NonNull<u8>);

    unsafe fn obj_skip<'a>(&self, reader: &mut StrReader<'a>) -> (ParseResult<()>, NonNull<u8>);

    fn obj_brigadier(&self) -> Option<pkt::Parser<'static>>;

    /// # Safety
    /// Suggestions must be dropped and suggestions pointer must point to valid
    /// suggestions object.
    unsafe fn obj_call_suggestions(
        &self,
        suggestions: NonNull<u8>,
        real: RealCommandExecutor,
        transaction: SuggestionsTransaction,
        executor: CommandExecutor,
        answer: &mut SuggestionAnswerer,
        command: String,
        world: &World,
    );

    unsafe fn obj_drop_suggestions(&self, suggestions: NonNull<u8>);
}

pub(crate) struct ParseWithData<'a, T: Parse<'a>>(pub T::Data);

impl<T: Parse<'static> + RawParseSuggestions<'static>> ParseObject
    for ParseWithData<'static, T>
{
    unsafe fn obj_parse<'a>(
        &self,
        reader: &mut StrReader<'a>,
        fill: &mut ParseResultsWrite,
    ) -> (ParseResult<()>, NonNull<u8>) {
        let mut suggestions = T::Suggestions::default();
        let result = T::parse(&self.0, &mut suggestions, unsafe {
            std::mem::transmute(reader)
        });
        (
            match result {
                Ok(obj) => {
                    fill.write(obj);
                    Ok(())
                }
                Err(e) => Err(e),
            },
            // SAFETY: drop provided by next methods
            unsafe { std::mem::transmute(Box::new(suggestions)) },
        )
    }

    unsafe fn obj_skip<'a>(&self, reader: &mut StrReader<'a>) -> (ParseResult<()>, NonNull<u8>) {
        let mut suggestions = T::Suggestions::default();
        let result = T::skip(&self.0, &mut suggestions, unsafe {
            std::mem::transmute(reader)
        });
        (
            result,
            // SAFETY: drop provided by next methods
            unsafe { std::mem::transmute(Box::new(suggestions)) },
        )
    }

    fn obj_brigadier(&self) -> Option<pkt::Parser<'static>> {
        T::brigadier(&self.0)
    }

    unsafe fn obj_call_suggestions(
        &self,
        suggestions: NonNull<u8>,
        real: RealCommandExecutor,
        transaction: SuggestionsTransaction,
        executor: CommandExecutor,
        answer: &mut SuggestionAnswerer,
        command: String,
        world: &World,
    ) {
        // SAFETY: Box is a boxed NonNull and method accepts only valid T::Suggestions
        // pointers
        let suggestions: Box<T::Suggestions> = std::mem::transmute(suggestions);
        let suggestions = *suggestions;
        T::call_suggestions(
            &self.0,
            real,
            transaction,
            executor,
            answer,
            suggestions,
            command,
            world,
        )
    }

    unsafe fn obj_drop_suggestions(&self, suggestions: NonNull<u8>) {
        // SAFETY: provided by caller
        let _: Box<T::Suggestions> = std::mem::transmute(suggestions);
    }
}
