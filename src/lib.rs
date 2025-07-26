use proc_macro::TokenStream;
use quote::quote;
use std::env;
use std::path::PathBuf;
use syn::LitStr;

#[proc_macro]
pub fn embed_image(input: TokenStream) -> TokenStream {
    let path = syn::parse_macro_input!(input as LitStr);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let full_path: PathBuf = PathBuf::from(manifest_dir).join(path.value());

    let image = image::open(&full_path)
        .expect("Failed to open image")
        .into_luma8();

    let mut img = image.clone();
    image::imageops::colorops::dither(&mut img, &image::imageops::colorops::BiLevel);

    let width = img.width() as usize;
    let height = img.height() as usize;
    let mut packed: Vec<u8> = Vec::with_capacity((width * height + 7) / 8);

    for y in 0..height {
        let row_start = y * width;
        let row = &img.as_raw()[row_start..row_start + width];
        let mut byte = 0u8;
        for (i, &pixel) in row.iter().enumerate() {
            if pixel == 0 {
                byte |= 1 << (7 - (i % 8));
            }
            if i % 8 == 7 {
                packed.push(byte);
                byte = 0;
            }
        }
        if width % 8 != 0 {
            packed.push(byte);
        }
    }

    let data_tokens = packed.iter().map(|b| quote! { #b });
    let width_const: u16 = width as u16;
    let height_const: u16 = height as u16;

    let out = quote! {{
        const DATA: &[u8] = &[ #(#data_tokens),* ];
        escpos_embedded::Image {
            width: #width_const,
            height: #height_const,
            data: DATA,
        }
    }};
    out.into()
}
