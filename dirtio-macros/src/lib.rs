use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// A simple procedure macro, providing a shorthand
/// to set up the runtime. The default setting is used.
#[proc_macro_attribute]
pub fn main(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input: ItemFn = parse_macro_input!(item);

    let fn_name = &input.sig.ident;
    let fn_output = &input.sig.output;
    let fn_args = &input.sig.inputs;
    let fn_body = &input.block;

    let gen = quote! {
        fn #fn_name(#fn_args) #fn_output {
            let rt = dirtio::runtime::Builder::new()
                .build()
                .expect("failed to build runtime");
            rt.block_on(async move { #fn_body })
        }
    };

    gen.into()
}
