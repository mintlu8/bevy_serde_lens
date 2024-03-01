use proc_macro2::{Span, TokenStream};
use proc_macro_error::{abort, proc_macro_error};
use syn::{spanned::Spanned, Attribute, DataEnum, DeriveInput, Error, Expr, Field, Generics, Ident, Lit, LitStr, MetaNameValue, Type};
use quote::{format_ident, quote};

/// Project a struct to and from a (de)serializable struct using `World` access.
/// Requires all fields with `SerdeProject` or `Serialize` + `DeserializeOwned` implementations.
/// 
/// # Attributes
/// * `#[serde_project("TypeName")]`
///
///     Convert to and from the target `SerdeProject` type via `bevy_serde_project::Convert`.
///     Commonly used to convert a foreign type to a newtype.
///
/// * `#[serde_project(ignore)]`
/// 
///     Ignore the field and default construct it if deserialized.
///
/// * `#[serde_project(rename = "new_name")]`
/// 
///     Top level `#[serde(rename)]` is not allowed, this attribute replaces that functionality.
///
/// * `#[serde(..)]`
///
///     Copied to destination.
#[proc_macro_error]
#[proc_macro_derive(SerdeProject, attributes(serde_project, serde))]
pub fn derive_project(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    derive_project2(stream.into()).unwrap().into()
}

fn into_ser(ty: &Type, expr: TokenStream) -> TokenStream {
    quote!(
        <#ty as __bsp::SerdeProject>::to_ser(
            #expr, 
            &__bsp::from_world::<#ty>(__ctx)?
        )?
    )
}

fn from_de(ty: &Type, expr: TokenStream) -> TokenStream {
    quote!(
        <#ty as __bsp::SerdeProject>::from_de(
            &mut __bsp::from_world_mut::<#ty>(__ctx)?,
            #expr
        )?
    )
}

fn derive_project2(stream: TokenStream) -> Result<TokenStream, syn::Error> {
    let input = syn::parse2::<DeriveInput>(stream)?;
    let generics = input.generics;
    let name = input.ident;
    let attrs = input.attrs;
    Ok(match input.data {
        syn::Data::Union(union) => {
            abort!(union.union_token.span, "Union is not supported")
        },
        syn::Data::Struct(data_struct) => match data_struct.fields {
            syn::Fields::Unit => {
                abort!(Span::call_site(), "'SerdeProject' is not needed on a unit struct.\nPlease derive or implement 'Serialize' and 'Deserialize' directly.")
            },
            syn::Fields::Named(named_fields) => {
                parse_struct_input(name, generics, attrs, named_fields.named, true)?
            },
            syn::Fields::Unnamed(unnamed_fields) => {
                parse_struct_input(name, generics, attrs, unnamed_fields.unnamed, false)?
            },
        },
        syn::Data::Enum(data_enum) => {
            parse_enum(name, generics, attrs, data_enum)?
        },
    })
}

fn parse_struct_input(name: Ident, generics: Generics, attrs: Vec<Attribute>, fields: impl IntoIterator<Item=Field>, named: bool) -> Result<TokenStream, Error> {
    let (impl_generics, ty_genetics, where_clause) = generics.split_for_impl();

    let (name_str, head_attrs) = parse_head_attrs(attrs)?;
    let name_str = name_str.unwrap_or(LitStr::new(&name.to_string(), name.span()));
    let ParsedStruct {
        ser_ty, 
        de_ty, 
        original_decomp, 
        ser_construct, 
        de_decomp, 
        original_construct 
    } = parse_struct(fields, named)?;

    let sep = if named {quote!()} else {quote!(;)};

    Ok(quote!(
        #[allow(unused)]
        const _: () = {
            use ::core::borrow::Borrow;
            use ::bevy_serde_project as __bsp;
            #[derive(__bsp::serde::Serialize)]
            #(#head_attrs)*
            #[serde(rename = #name_str)]
            pub struct __Ser<'t> #ser_ty #sep
            #[derive(__bsp::serde::Deserialize)]
            #(#head_attrs)*
            #[serde(rename = #name_str, bound="'t: 'de")]
            pub struct __De<'t> #de_ty #sep

            impl #impl_generics __bsp::SerdeProject for #name #ty_genetics #where_clause {
                type Ctx = __bsp::WorldAccess;

                type Ser<'s> = __Ser<'s>;
                type De<'de> = __De<'de>;

                fn to_ser<'t>(&'t self, __ctx: &&'t __bsp::World) -> Result<Self::Ser<'t>, Box<__bsp::Error>> {
                    let #name #original_decomp = self;
                    Ok(__Ser #ser_construct)
                }

                fn from_de(__ctx: &mut &mut __bsp::World, de: Self::De<'_>) -> Result<Self, Box<__bsp::Error>> {
                    let __De #de_decomp = de;
                    Ok(Self #original_construct)
                }
            }
        };
    ))
}

fn parse_enum(name: Ident, generics: Generics, attrs: Vec<Attribute>, variants: DataEnum) -> Result<TokenStream, Error> {
    let (impl_generics, ty_genetics, where_clause) = generics.split_for_impl();

    let (name_str, head_attrs) = parse_head_attrs(attrs)?;
    let name_str = name_str.unwrap_or(LitStr::new(&name.to_string(), name.span()));

    let mut ser_fields = Vec::new();
    let mut de_fields = Vec::new();

    let mut ser_branches = Vec::new();
    let mut de_branches = Vec::new();
    let mut ignored_branches = Vec::new();
    let mut serde_attrs = Vec::new();

    for variant in variants.variants {
        let (attr_result, serde_result) = parse_attrs(variant.attrs)?;
        let variant_name = variant.ident;
        match attr_result {
            AttrResult::None => (),
            AttrResult::Project(ty) => abort!(ty.span(), "Conversion is not allowed here."),
            AttrResult::Ignore => {
                let name = variant_name.to_string();
                ignored_branches.push(match variant.fields {
                    syn::Fields::Named(_) => quote!(
                        #name::#variant_name {..} => 
                            return Err(__bsp::Error::SkippedVariant(#name).boxed())
                    ),
                    syn::Fields::Unnamed(_) => quote!(
                        #name::#variant_name (..) => 
                            return Err(__bsp::Error::SkippedVariant(#name).boxed())
                    ),
                    syn::Fields::Unit => quote!(
                        #name::#variant_name => 
                            return Err(__bsp::Error::SkippedVariant(#name).boxed())
                    ),
                });
                continue;
            },
        }
        serde_attrs.push(serde_result);
        match variant.fields {
            syn::Fields::Named(fields) => {
                let ParsedStruct { 
                    ser_ty, 
                    de_ty, 
                    original_decomp, 
                    ser_construct, 
                    de_decomp, 
                    original_construct 
                } = parse_struct(fields.named, true)?;
                ser_fields.push(quote!(#variant_name #ser_ty));
                de_fields.push(quote!(#variant_name #de_ty));
                ser_branches.push(quote!(
                    #name::#variant_name #original_decomp => 
                        __Ser::#variant_name #ser_construct
                ));
                de_branches.push(quote!(
                    __De::#variant_name #de_decomp => 
                        #name::#variant_name #original_construct
                ));
            },
            syn::Fields::Unnamed(fields) => {
                let ParsedStruct { 
                    ser_ty, 
                    de_ty, 
                    original_decomp, 
                    ser_construct, 
                    de_decomp, 
                    original_construct 
                } = parse_struct(fields.unnamed, false)?;
                ser_fields.push(quote!(#variant_name #ser_ty));
                de_fields.push(quote!(#variant_name #de_ty));
                ser_branches.push(quote!(
                    #name::#variant_name #original_decomp => 
                        __Ser::#variant_name #ser_construct
                ));
                de_branches.push(quote!(
                    __De::#variant_name #de_decomp => 
                        #name::#variant_name #original_construct
                ));
            },
            syn::Fields::Unit => {
                ser_fields.push(quote!(#variant_name));
                de_fields.push(quote!(#variant_name));
                ser_branches.push(quote!(
                    #name::#variant_name => __Ser::#variant_name 
                ));
                de_branches.push(quote!(
                    __De::#variant_name => #name::#variant_name
                ));
            },
        }
    }

    if ser_fields.is_empty() {
        return Ok(quote!(
            const _: () = {
                use ::core::borrow::Borrow;
                use ::bevy_serde_project as __bsp;
                impl #impl_generics __bsp::SerdeProject for #name #ty_genetics #where_clause {
                    type Ctx = __bsp::NoContext;

                    type Ser<'s> = ();
                    type De<'de> = ();

                    fn to_ser<'t>(&'t self, __ctx: &()) -> Result<Self::Ser<'t>, Box<__bsp::Error>> {
                        Err(__bsp::Error::NoValidVariants.boxed())
                    }

                    fn from_de(__ctx: &mut (), de: Self::De<'_>) -> Result<Self, Box<__bsp::Error>> {
                        Err(__bsp::Error::NoValidVariants.boxed())
                    }
                }
            };
            
        ));
    }
    
    Ok(quote!(
        #[allow(unused)]
        const _: () = {
            use ::core::borrow::Borrow;
            use ::bevy_serde_project as __bsp;

            #[derive(__bsp::serde::Serialize)]
            #(#head_attrs)*
            #[serde(rename = #name_str)]
            pub enum __Ser<'t> {
                #(#(#serde_attrs)* #ser_fields,)*
                #[serde(skip)]
                __Phantom(&'t ::std::convert::Infallible)
            }

            #[derive(__bsp::serde::Deserialize)]
            #(#head_attrs)*
            #[serde(rename = #name_str, bound="'t: 'de")]
            pub enum __De<'t> {
                #(#(#serde_attrs)* #de_fields,)*
                #[serde(skip)]
                __Phantom(&'t ::std::convert::Infallible)
            }

            impl #impl_generics __bsp::SerdeProject for #name #ty_genetics #where_clause {
                type Ctx = __bsp::WorldAccess;

                type Ser<'s> = __Ser<'s>;
                type De<'de> = __De<'de>;

                fn to_ser<'t>(&'t self, __ctx: &&'t __bsp::World) -> Result<Self::Ser<'t>, Box<__bsp::Error>> {
                    Ok(match self {
                        #(#ser_branches,)*
                        #(#ignored_branches,)*
                    })
                }

                fn from_de(__ctx: &mut &mut __bsp::World, de: Self::De<'_>) -> Result<Self, Box<__bsp::Error>> {
                    Ok(match de {
                        #(#de_branches,)*
                        __De::__Phantom(_) => return Err(__bsp::Error::PhantomBranch.boxed())
                    })
                }
            }
        };
    ))
}

struct ParsedStruct {
    ser_ty: TokenStream,
    de_ty: TokenStream,
    original_decomp: TokenStream,
    ser_construct: TokenStream,
    de_decomp: TokenStream,
    original_construct: TokenStream,
}

enum AttrResult {
    None,
    Project(Type),
    Ignore
}

fn parse_attrs(attrs: Vec<Attribute>) -> syn::Result<(AttrResult, Vec<Attribute>)> {
    let mut serde_attrs = Vec::new();
    let mut result = AttrResult::None;
    for attr in attrs {
        if attr.meta.path().is_ident("serde") {
            serde_attrs.push(attr);
            continue;
        }
        if !attr.meta.path().is_ident("serde_project") {
            continue;
        }
        if !matches!(result, AttrResult::None) {
            abort!(attr.span(), "Repeat `serde_project` attributes are not allowed.")
        }
        let meta_list = attr.meta.require_list()?;
        if let Ok(string) = meta_list.parse_args::<LitStr>() {
            result = AttrResult::Project(string.parse::<Type>()?);
        } else if let Ok(ident) = meta_list.parse_args::<Ident>() {
            if ident == "ignore" {
                result = AttrResult::Ignore;
            } else {
                abort!(attr.span(), "Unable to parse this attribute.")
            }
        } else {
            abort!(attr.span(), "Unable to parse this attribute.")
        }
    }
    Ok((result, serde_attrs))
}


fn parse_head_attrs(attrs: Vec<Attribute>) -> syn::Result<(Option<LitStr>, Vec<Attribute>)> {
    let mut serde_attrs = Vec::new();
    let mut result = None;
    for attr in attrs {
        if attr.meta.path().is_ident("serde") {
            serde_attrs.push(attr);
            continue;
        }
        if !attr.meta.path().is_ident("serde_project") {
            continue;
        }
        if result.is_none() {
            abort!(attr.span(), "Repeat `serde_project` attributes are not allowed.")
        }
        let meta_list = attr.meta.require_list()?;
        let name_value = meta_list.parse_args::<MetaNameValue>()?;
        
        if name_value.path.is_ident("parse") {
            if let Expr::Lit(lit) = name_value.value {
                if let Lit::Str(str) = lit.lit {
                    result = Some(str);
                    continue;
                }
            }
        } 
        abort!(attr.span(), "Unable to parse this attribute.")
    }
    Ok((result, serde_attrs))
}

struct NumberedIdents(usize);

impl Iterator for NumberedIdents {
    type Item = Ident;

    fn next(&mut self) -> Option<Self::Item> {
        let result = format_ident!("__field{}", self.0);
        self.0 += 1;
        Some(result)
    }
}

fn parse_struct(
    fields_iter: impl IntoIterator<Item=Field>,
    named: bool,
) -> Result<ParsedStruct, Error> {
    let mut original_decomp = Vec::new();
    let mut de_decomp = Vec::new();
    let mut ser_construct = Vec::new();
    let mut original_construct = Vec::new();

    let mut de_ty = Vec::new();
    let mut ser_ty = Vec::new();
    let mut serde_attrs = Vec::new();

    for (field, name) in fields_iter.into_iter().zip(NumberedIdents(0)) {
        let (arg_result, serde_attrs_field) = parse_attrs(field.attrs)?;
        let name = field.ident.unwrap_or(name);
        original_decomp.push(name.clone());

        match arg_result {
            AttrResult::Ignore => {
                original_construct.push(quote!(Default::default()))
            },
            AttrResult::None => {
                let ty = field.ty;
                de_decomp.push(name.clone());
                serde_attrs.push(serde_attrs_field);
                ser_ty.push(quote!(__bsp::Ser<'t, #ty>));
                de_ty.push(quote!(__bsp::De<'t, #ty>));
                ser_construct.push(into_ser(&ty, quote!(#name)));
                original_construct.push(from_de(&ty, quote!(#name)));
            },
            AttrResult::Project(ty) => {
                let src_ty = field.ty;
                de_decomp.push(name.clone());
                serde_attrs.push(serde_attrs_field);
                ser_ty.push(quote!(__bsp::Ser<'t, #ty>));
                de_ty.push(quote!(__bsp::De<'t, #ty>));
                ser_construct.push(into_ser(&ty, quote!(
                    <#ty as __bsp::Convert<#src_ty>>::ser(#name).borrow()
                )));
                let ty_construct = from_de(&ty, quote!(#name));
                original_construct.push(quote!(
                    <#ty as __bsp::Convert<#src_ty>>::de(#ty_construct)
                ));
            },
        }
    }
    if named {
        Ok(ParsedStruct {
            ser_ty: quote!({
                #(#(#serde_attrs)* #de_decomp: #ser_ty,)*
                #[serde(skip)]
                __p: ::std::marker::PhantomData<&'t ()>
            }),
            de_ty: quote!({
                #(#(#serde_attrs)* #de_decomp: #de_ty,)*
                #[serde(skip)]
                __p: ::std::marker::PhantomData<&'t ()>
            }),
            original_decomp: quote!({
                #(#original_decomp,)*
            }),
            de_decomp: quote!({
                #(#de_decomp,)*
                __p
            }),
            ser_construct: quote!({
                #(#de_decomp: #ser_construct,)*
                __p: ::std::marker::PhantomData
            }),
            original_construct: quote!({
                #(#original_decomp: #original_construct,)*
            }),
        })
    } else {
        Ok(ParsedStruct {
            ser_ty: quote!((
                #(#ser_ty,)*
                ::std::marker::PhantomData<&'t ()>
            )),
            de_ty: quote!((
                #(#de_ty,)*
                ::std::marker::PhantomData<&'t ()>
            )),
            original_decomp: quote!((
                #(#original_decomp,)*
            )),
            de_decomp: quote!((
                #(#de_decomp,)*
                _
            )),
            ser_construct: quote!((
                #(#ser_construct,)*
                ::std::marker::PhantomData
            )),
            original_construct: quote!((
                #(#original_construct,)*
            )),
        })
    }
}
