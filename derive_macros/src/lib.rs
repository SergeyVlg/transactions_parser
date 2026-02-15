use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};
use convert_case::{Case, Casing};

#[proc_macro_derive(GenerateProcessedFields)]
pub fn generate_processed_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("Macro only supports structs with named fields"),
        },
        _ => panic!("Macro only supports structs"),
    };

    let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    // Генерируем имена констант в PascalCase (например, from_user_id -> FromUserId)
    let const_names: Vec<_> = field_names
        .iter()
        .map(|f| {
            let name = f.as_ref().unwrap().to_string().to_case(Case::Pascal);
            syn::Ident::new(&name, f.as_ref().unwrap().span())
        })
        .collect();

    let indices: Vec<_> = (0..field_names.len()).collect();
    let count = field_names.len();

    // Генерируем итоговый код
    let expanded = quote! {
        bitflags::bitflags! {
            struct ProcessedFields : u32 {
                #(
                    const #const_names = 1 << #indices;
                )*
            }
        }
    };

    TokenStream::from(expanded)
}
