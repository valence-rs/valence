use proc_macro2::TokenStream;
use quote::quote;

const MAX_VIEW_DIST: u8 = 32; // This can be increased to 64 if we want.
const EXTRA_VIEW_RADIUS: i32 = 2;

pub fn build() -> TokenStream {
    let entries = (0..=MAX_VIEW_DIST).map(|dist| {
        let dist = dist as i32 + EXTRA_VIEW_RADIUS;

        let mut positions = vec![];

        for z in -dist..=dist {
            for x in -dist..=dist {
                if x * x + z * z <= dist * dist {
                    positions.push((x as i8, z as i8));
                }
            }
        }

        positions.sort_by_key(|&(x, z)| (x as i32).pow(2) + (z as i32).pow(2));

        let array_elems = positions.into_iter().map(|(x, z)| quote!((#x, #z)));

        quote! {
            &[ #(#array_elems),* ]
        }
    });

    let array_len = MAX_VIEW_DIST as usize + 1;

    quote! {
        /// The maximum view distance for a [`ChunkView`].
        pub const MAX_VIEW_DIST: u8 = #MAX_VIEW_DIST;

        const EXTRA_VIEW_RADIUS: i32 = #EXTRA_VIEW_RADIUS;

        static CHUNK_VIEW_LUT: [&[(i8, i8)]; #array_len] = [ #(#entries),* ];
    }
}
