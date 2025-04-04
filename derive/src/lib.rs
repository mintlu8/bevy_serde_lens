use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream, TokenTree};
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, Data, DeriveInput, Expr, Lit, Meta, Path,
    Token,
};

/// Derive macro for `BevyObject`. This largely mirrors `Bundle` but supports additional types of fields.
///
/// * `impl BevyObject` contains another `BevyObject` on the same entity.
/// * `Maybe<T>` makes existence of `T` optional and maps to an `Option`.
/// * `DefaultInit<T>` initializes a non-serialize component with `FromWorld` during deserialization.
/// * `Child<T>` inserts/finds a single child `BevyObject` during de/serialization.
/// * `ChildVec<T>` inserts/finds multiple children `BevyObject` during de/serialization.
///
/// `Component` is automatically `BevyObject` so no need to implement on them.
///
/// # Top Level Attributes
///
/// * `#[bevy_object(query)]`
///
/// Assert the type can be serialized from a single query (no children, i.e `Child` and `ChildVec`).
/// This speeds up serialization.
///
/// * `#[bevy_object(rename = "Name")]`
///
/// Change the serialized name of this type.
///
/// * `#[bevy_object(parent = "function")]`
///
/// Provide a function to obtain a parent `Entity` to parent this to, optional.
///
/// Expects `fn(&mut World) -> Option<EntityWorldMut>`.
///
/// # Field Attributes
///
/// * `#[bevy_object(no_filter)]`
///
/// Ignore the `QueryFilter` generated by this field,
/// this is useful for validating data integrity during serialization.
///
/// # Serde Attributes
///
/// You can specify serde attributes `#[serde]` in this macro but you don't need to actually derive serde.
/// The type used with serde is generated separately and `#[serde]` attributes are mirrored.
///
/// If you have a component `Transform`, the type serialized is actually a ZST called `SerializeComponent<Transform>`.
/// this means serde attributes like `skip_serializing_if` cannot be used directly.
///
/// `#[serde(skip)]` should be used on `DefaultInit`
/// and `#[serde(default)]` is supported on `Maybe`.
#[proc_macro_error]
#[proc_macro_derive(BevyObject, attributes(bevy_object, serde))]
pub fn serialization_archetype(tokens: TokenStream1) -> TokenStream1 {
    serialization_archetype2(tokens.into()).into()
}

fn token_stream_is_ident(stream: &TokenStream, name: &str) -> bool {
    let mut iter = stream.clone().into_iter();
    let Some(TokenTree::Ident(ident)) = iter.next() else {
        return false;
    };
    let None = iter.next() else {
        return false;
    };
    ident == name
}

fn parse_attr(attr: &Attribute, name: &str) -> bool {
    match &attr.meta {
        Meta::List(list) => {
            if list.path.get_ident().is_some_and(|i| i == "bevy_object") {
                token_stream_is_ident(&list.tokens, name)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn parse_attr_main(
    attr: &Attribute,
    query: &mut bool,
    name: &mut String,
    parent: &mut Option<Path>,
) {
    let Meta::List(list) = &attr.meta else { return };
    if list.path.get_ident().is_none_or(|i| i != "bevy_object") {
        return;
    };
    let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
        return;
    };
    for meta in nested {
        match meta {
            Meta::Path(path) if path.is_ident("query") => {
                *query = true;
            }
            Meta::NameValue(meta) if meta.path.is_ident("rename") => {
                let Expr::Lit(lit) = meta.value else { continue };
                let Lit::Str(lit) = lit.lit else { continue };
                *name = lit.value();
            }
            Meta::NameValue(meta) if meta.path.is_ident("parent") => {
                let Expr::Lit(lit) = meta.value else { continue };
                let Lit::Str(lit) = lit.lit else { continue };
                *parent = syn::parse_str(&lit.value()).ok();
            }
            _ => (),
        }
    }
}

fn is_forwarded(attr: &Attribute) -> bool {
    match &attr.meta {
        Meta::List(list) => list.path.get_ident().is_some_and(|i| i == "serde"),
        _ => false,
    }
}

fn roll_tuple<T: ToTokens>(types: &[T]) -> TokenStream {
    let mut result = quote! {()};
    for item in types {
        result = quote! {(#item, #result)};
    }
    result
}

fn serialization_archetype2(tokens: TokenStream) -> TokenStream {
    let Ok(result) = syn::parse2::<DeriveInput>(tokens) else {
        abort!(Span::call_site(), "Invalid input.")
    };

    let Data::Struct(st) = result.data else {
        abort!(result.span(), "Invalid struct.")
    };

    let name = result.ident;
    let mut name_str = name.to_string();
    let mut parent = None;
    let mut is_query = false;

    for attr in &result.attrs {
        parse_attr_main(attr, &mut is_query, &mut name_str, &mut parent);
    }

    let name_binding = format_ident!("{name}Binding");
    let mut fields = Vec::new();
    let mut types = Vec::new();
    let mut types_query = Vec::new();
    let mut filters = Vec::new();
    let mut queries = Vec::new();
    let main_attrs: Vec<_>;
    let mut field_attrs = Vec::<Vec<_>>::new();
    main_attrs = result.attrs.into_iter().filter(is_forwarded).collect();

    let crate0 = quote! {::bevy_serde_lens};

    for field in st.fields {
        let Some(name) = field.ident else {
            abort!(field.span(), "Tuple struct is not supported.")
        };
        let ty = field.ty;
        fields.push(name);
        types.push(quote! {
            <#ty as #crate0::BindProject>::To
        });
        if is_query {
            types_query.push(quote! {
                #crate0::BindItem<'t, #ty>
            })
        }
        if !field.attrs.iter().any(|x| parse_attr(x, "no_filter")) {
            filters.push(quote! {
                <#ty as #crate0::BindProject>::Filter
            });
        }
        queries.push(quote! {<#ty as #crate0::BindProjectQuery>::Data});
        field_attrs.push(field.attrs.into_iter().filter(is_forwarded).collect())
    }

    let filter = roll_tuple(&filters);

    let data = if is_query {
        roll_tuple(&queries)
    } else {
        quote!(())
    };
    let mut ext = TokenStream::new();

    if is_query {
        let rolled_fields = roll_tuple(&fields);
        ext.extend(quote! {
            fn into_ser(query_data: #crate0::Item<'_, Self>) -> impl #crate0::serde::Serialize{
                let #rolled_fields = query_data;
                #[derive(#crate0::serde::Serialize)]
                #(#main_attrs)*
                struct #name<'t> {
                    #(#(#field_attrs)* #fields: #types_query,)*
                }
                #name {
                    #(#fields),*
                }
            }
        })
    }

    if let Some(parent) = parent {
        ext.extend(quote! {
            fn get_root(world: &mut #crate0::World) -> Option<#crate0::EntityWorldMut> {
                #parent()
            }
        })
    }

    quote!(
        const _: () = {
            #[derive(#crate0::serde::Serialize, #crate0::serde::Deserialize)]
            #(#main_attrs)*
            pub struct #name_binding {
                #(#(#field_attrs)* #fields: #types,)*
            }

            impl #crate0::ZstInit for #name_binding {
                fn init() -> Self {
                    Self {
                        #(#fields: #crate0::ZstInit::init(),)*
                    }
                }
            }

            impl #crate0::BevyObject for #name {
                const IS_QUERY: bool = #is_query;
                type Data = #data;
                type Filter = #filter;
                type Object = #name_binding;

                fn name() -> &'static str {
                    #name_str
                }

                #ext
            }

        };
    )
}
