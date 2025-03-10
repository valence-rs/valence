use proc_macro2::TokenStream;
use quote::quote;

const MAX_VIEW_DIST: u8 = 32;
const EXTRA_VIEW_RADIUS: i32 = 2;

pub(crate) fn build() -> TokenStream {
    let entries = (0..=MAX_VIEW_DIST).map(|dist| {
        let dist = i32::from(dist) + EXTRA_VIEW_RADIUS;

        let mut positions = vec![];

        for z in -dist..=dist {
            for x in -dist..=dist {
                if x * x + z * z <= dist * dist {
                    positions.push((x as i8, z as i8));
                }
            }
        }

        positions.sort_by_key(|&(x, z)| i32::from(x).pow(2) + i32::from(z).pow(2));

        let array_elems = positions.into_iter().map(|(x, z)| quote!((#x, #z)));

        quote! {
            &[ #(#array_elems),* ]
        }
    });

    let array_len = MAX_VIEW_DIST as usize + 1;

    quote! {
        #[doc = "The maximum view distance for a `ChunkView`."]
        pub const MAX_VIEW_DIST: u8 = #MAX_VIEW_DIST;

        pub const EXTRA_VIEW_RADIUS: i32 = #EXTRA_VIEW_RADIUS;

        pub static CHUNK_VIEW_LUT: [&[(i8, i8)]; #array_len] = [ #(#entries),* ];
    }
}
