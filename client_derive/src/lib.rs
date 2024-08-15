use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// # Check duplicates derive macro
///
/// Implements the `has_duplicates` method for given struct. The method iterates
/// over all fields of the struct, inserts the values into a temporary `HashSet`
/// and checks for duplicate values.
///
/// By using this macro we can extend a struct without having to worry about adding
/// the new field to the duplicate check.
#[proc_macro_derive(CheckDuplicates)]
pub fn derive_check_duplicates(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Match the input struct and iterate over its fields
    let check_duplicates_impl = match input.data {
        Data::Struct(data_struct) => {
            let fields = match &data_struct.fields {
                Fields::Named(fields) => &fields.named,
                _ => panic!("CheckDuplicates can only be derived for structs with named fields."),
            };

            // Create an iterator over the fields and generate code to insert into HashSet
            let field_checks = fields.iter().map(|field| {
                let field_name = &field.ident;
                quote! {
                    if !set.insert(&self.#field_name) {
                        return true;
                    }
                }
            });

            quote! {
                impl #name {
                    fn has_duplicates(&self) -> bool {
                        let mut set = std::collections::HashSet::new();
                        #(#field_checks)*
                        false
                    }
                }
            }
        }
        _ => panic!("CheckDuplicates can only be derived for structs."),
    };

    TokenStream::from(check_duplicates_impl)
}

/// # Check children duplicates derive macro
///
/// Implements the `children_have_duplicates` method for given struct.
/// The method iterates over all fields of the struct and calls
/// `field.has_duplicates()` generated via the `CheckDuplicates` derive macro.
/// Whenever a field has duplicates, return true. Otherwise return false.
///
/// By using this macro we can extend a struct without having to worry about adding
/// the new field to the duplicate check.
#[proc_macro_derive(CheckChildrenDuplicates)]
pub fn derive_has_duplicates(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Match the input struct and iterate over its fields
    let check_duplicates_impl = match input.data {
        Data::Struct(data_struct) => {
            let fields = match data_struct.fields {
                Fields::Named(ref fields) => &fields.named,
                _ => panic!("IsValid can only be derived for structs with named fields."),
            };

            // Create an iterator over the fields and check for each field whether it has
            // duplicate keybindings.
            let field_checks = fields.iter().map(|field| {
                let field_name = &field.ident;
                quote! {
                    if self.#field_name.has_duplicates() {
                        return true;
                    }
                }
            });

            quote! {
                impl KeyBindings {
                    pub fn children_have_duplicates(&self) -> bool {
                        #(#field_checks)*
                        false
                    }
                }
            }
        }
        _ => panic!("CheckDuplicates can only be derived for structs."),
    };

    TokenStream::from(check_duplicates_impl)
}
