use proc_macro2::TokenStream;
use proc_macro_error::abort;
use syn::{spanned::Spanned, DeriveInput, Ident, LitStr, Type};
use quote::quote;

/// Project a struct to and from a ser/de-able struct using `World` access.
/// Requires all fields with `SerdeProject` or `Serialize` + `DeserializeOwned` implementations.
/// 
/// # Attributes
/// * `#[serde_project("TypeName")]`
///
///     Convert to and from the target `SerdeProject` type via `Convert`.
///     Commonly used to convert a foreign type to a newtype.
///
/// * `#[serde_project(ignore)]`
/// 
///     Ignore the field and default construct it.
///
/// * `#[serde(..)]`
///
///     Copied to destination.
#[proc_macro_derive(SerdeProject, attributes(serde_project, serde))]
pub fn derive_project(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_project2(stream.into()).unwrap().into()
}

fn into_ser(ty: &Type, expr: TokenStream) -> TokenStream {
    quote!(
        <#ty as __bsp::SerdeProject>::to_ser(
            #expr, 
            __bsp::from_world::<#ty>(ctx)?
        )?
    )
}

fn from_de(ty: &Type, expr: TokenStream) -> TokenStream {
    quote!(
        <#ty as __bsp::SerdeProject>::from_de(
            __bsp::from_world_mut::<#ty>(ctx)?,
            #expr
        )?
    )
}

fn derive_project2(stream: TokenStream) -> Result<TokenStream, syn::Error> {
    let input = syn::parse2::<DeriveInput>(stream)?;
    let (impl_generics, ty_genetics, where_clause) = input.generics.split_for_impl();
    let name = input.ident;
    Ok(match input.data {
        syn::Data::Union(union) => {
            abort!(union.union_token.span, "Union is not supported")
        },
        syn::Data::Struct(data_struct) => match data_struct.fields {
            syn::Fields::Unit => {
                abort!(name.span(), "Derive 'Serialize' and 'Deserialize' instead.")
            },
            syn::Fields::Named(named_fields) => {
                let mut fields = Vec::new();
                let mut types = Vec::new();
                let mut froms = Vec::new();
                let mut intos = Vec::new();
                let mut serde_attrs = Vec::new();

                let mut ignored = Vec::new();
                'main: for field in named_fields.named {
                    let name = field.ident.unwrap();
                    let mut ty = field.ty;
                    let mut into = into_ser(&ty, quote!(&self.#name));
                    let mut from = from_de(&ty, quote!(de.#name));
                    let mut found = false;
                    let mut serde_attrs_field = Vec::new();
                    for attr in field.attrs {
                        if attr.meta.path().is_ident("serde") {
                            serde_attrs_field.push(attr);
                            continue;
                        }
                        if !attr.meta.path().is_ident("serde_project") {
                            continue;
                        }
                        if found {
                            abort!(attr.span(), "Repeat `serde_project` attributes are not allowed.")
                        }
                        found = true;
                        let meta_list = attr.meta.require_list()?;
                        if let Ok(string) = meta_list.parse_args::<LitStr>() {
                            let replace = string.parse::<Type>()?;
                            into = into_ser(&replace, quote!(
                                <#replace as __bsp::Convert::<#ty>>::ser(&self.#name).borrow()
                            ));
                            let from_inner = from_de(&replace, quote!(de.#name));
                            from = quote!(<#replace as __bsp::Convert::<#ty>>::de(#from_inner));
                            ty = replace;
                        } else if let Ok(ident) = meta_list.parse_args::<Ident>() {
                            if ident == "ignore" {
                                ignored.push(name);
                                continue 'main;
                            }
                            abort!(ident.span(), "Unable to parse this attribute.")
                        } else {
                            abort!(attr.span(), "Unable to parse this attribute.")
                        }
                    }
                    fields.push(name);
                    types.push(ty);
                    froms.push(from);
                    intos.push(into);
                    serde_attrs.push(serde_attrs_field)
                }

                quote! (
                    const _: () = {
                        use ::core::borrow::Borrow;
                        use ::bevy_serde_project as __bsp;
                        #[derive(__bsp::serde::Serialize)]
                        pub struct __Ser<'s> {
                            #(#(#serde_attrs)* #fields: __bsp::Ser<'s, #types>,)*
                            __p: ::std::marker::PhantomData<&'s ()>
                        }
                        #[derive(__bsp::serde::Deserialize)]
                        pub struct __De<'d> {
                            #(#(#serde_attrs)* #fields: __bsp::De<'d, #types>,)*
                            __p: ::std::marker::PhantomData<&'d ()>
                        }

                        impl #impl_generics __bsp::SerdeProject for #name #ty_genetics #where_clause {
                            type Ctx = __bsp::WorldAccess;

                            type Ser<'s> = __Ser<'s>;
                            type De<'de> = __De<'de>;

                            fn to_ser<'t>(&'t self, ctx: &__bsp::World) -> Result<Self::Ser<'t>, Box<__bsp::Error>> {
                                Ok(__Ser {
                                    #(#fields: #intos,)*
                                    __p: ::std::marker::PhantomData
                                })
                            }

                            fn from_de<'de>(ctx: &mut __bsp::World, de: Self::De<'de>) -> Result<Self, Box<__bsp::Error>> {
                                Ok(Self {
                                    #(#fields: #froms,)*
                                    #(#ignored: Default::default(),)*
                                })
                            }
                        }
                    };
                )
            },
            syn::Fields::Unnamed(_) => todo!(),
        },
        syn::Data::Enum(_) => todo!(),
    })
}
