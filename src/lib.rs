use glob::glob;
use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;
use syn::{parse::Parse, parse::ParseStream, Attribute, Ident, LitStr, Token};

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

struct EmbedImagesInput {
    enum_ident: Ident,
    patterns: Vec<LitStr>,
}

impl Parse for EmbedImagesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![enum]>()?;
        let enum_ident: Ident = input.parse()?;
        let content;
        syn::braced!(content in input);
        let attrs: Vec<Attribute> = content.call(Attribute::parse_outer)?;
        if !content.is_empty() {
            return Err(content.error("unexpected tokens inside enum"));
        }
        let mut patterns = Vec::new();
        for attr in attrs {
            if attr.path().is_ident("pattern") {
                let lit: LitStr = attr.parse_args()?;
                patterns.push(lit);
            } else {
                return Err(syn::Error::new_spanned(attr, "unsupported attribute"));
            }
        }
        Ok(EmbedImagesInput {
            enum_ident,
            patterns,
        })
    }
}

#[proc_macro]
pub fn embed_images(input: TokenStream) -> TokenStream {
    let EmbedImagesInput {
        enum_ident,
        patterns,
    } = syn::parse_macro_input!(input as EmbedImagesInput);
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let mut seen = HashSet::new();
    let mut variants = Vec::new();
    let mut consts = Vec::new();
    let mut arms = Vec::new();

    for pattern in patterns {
        let full_pattern: PathBuf = PathBuf::from(&manifest_dir).join(pattern.value());
        for entry in
            glob(full_pattern.to_str().expect("invalid pattern")).expect("failed to read glob")
        {
            let path = entry.expect("glob path error");
            let file_stem = path.file_stem().expect("no file name").to_string_lossy();
            let variant_name = file_stem.to_upper_camel_case();
            if !seen.insert(variant_name.clone()) {
                continue;
            }
            let variant_ident = syn::Ident::new(&variant_name, Span::call_site());
            let const_ident = syn::Ident::new(&file_stem.to_shouty_snake_case(), Span::call_site());

            let rel_path = path.strip_prefix(&manifest_dir).unwrap_or(&path);
            let rel_path_str = rel_path.to_string_lossy();

            consts.push(quote! {
                static #const_ident: escpos_embedded::Image<&'static [u8]> = embed_image!(#rel_path_str);
            });
            variants.push(quote! { #variant_ident });
            arms.push(quote! { #enum_ident::#variant_ident => &#const_ident });
        }
    }

    let out = quote! {
        #(#consts)*

        pub enum #enum_ident {
            #(#variants),*
        }

        impl #enum_ident {
            pub const fn get_image(&self) -> &'static escpos_embedded::Image<&'static [u8]> {
                match self {
                    #(#arms),*
                }
            }
        }
    };

    out.into()
}
