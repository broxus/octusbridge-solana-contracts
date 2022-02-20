use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(bridge_pack), forward_attrs(allow, doc, cfg))]
struct Opts {
    length: usize,
}

#[proc_macro_derive(BridgePack, attributes(bridge_pack))]
pub fn derive_known_param_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    impl_derive_bridge_pack(input).into()
}

fn impl_derive_bridge_pack(input: syn::DeriveInput) -> TokenStream {
    let opts = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, .. } = input;

    let len = opts.length;

    let output = quote! {
        impl Pack for #ident {
            const LEN: usize = #len;

            fn pack_into_slice(&self, dst: &mut [u8]) {
                let mut data = self.try_to_vec().unwrap();
                let (left, _) = dst.split_at_mut(data.len());
                left.copy_from_slice(&mut data);
            }

            fn unpack_from_slice(mut src: &[u8]) -> Result<Self, ProgramError> {
                let unpacked = Self::deserialize(&mut src)?;
                Ok(unpacked)
            }
        }
    };
    output.into()
}
