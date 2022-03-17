#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parse, parse_macro_input, punctuated::Punctuated, spanned::Spanned, BareFnArg, Expr,
    Ident, Token, Type, Type::BareFn,
};

struct HookMacro {
    target: Expr,
    target_ty: Type,
    custom: Expr,
}

impl Parse for HookMacro {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let target: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let target_ty: Type = input.parse()?;
        input.parse::<Token![,]>()?;
        let custom: Expr = input.parse()?;

        Ok(HookMacro {
            target,
            target_ty,
            custom,
        })
    }
}

#[proc_macro]
pub fn make_hook(input: TokenStream) -> TokenStream {
    let HookMacro {
        target,
        target_ty,
        custom,
    } = parse_macro_input!(input as HookMacro);

    if let BareFn(fn_ty) = target_ty {
        if fn_ty.variadic.is_some() {
            fn_ty
                .span()
                .unwrap()
                .error("variadic functions aren't supported")
                .emit();
            return TokenStream::new();
        }

        let detour_unsafe = fn_ty.unsafety;
        let detour_abi = fn_ty.abi.clone();
        let detour_ret = fn_ty.output.clone();
        let detour_args = Punctuated::<BareFnArg, Token![,]>::from_iter(
            fn_ty.inputs.iter().enumerate().map(|(i, x)| BareFnArg {
                attrs: x.attrs.clone(),
                name: Some((
                    Ident::new(&format!("__arg_{}", i), Span::call_site()),
                    <Token![:]>::default(),
                )),
                ty: x.ty.clone(),
            }),
        );
        let detour_ffi_args = Punctuated::<Ident, Token![,]>::from_iter(
            (0..fn_ty.inputs.len()).map(|i| Ident::new(&format!("__arg_{}", i), Span::call_site())),
        );

        let expanded = quote! {
            {
                #[allow(non_upper_case_globals)]
                static _hook: ::detour::StaticDetour<#fn_ty> = {
                    #[inline(never)]
                    #[allow(unused_unsafe)]

                    #detour_unsafe #detour_abi fn __ffi_detour(#detour_args) #detour_ret {
                        #[allow(unused_unsafe)]
                        (_hook.__detour())(#detour_ffi_args)
                    }
                    ::detour::StaticDetour::__new(__ffi_detour)
                };
                _hook.initialize(#target, #custom)?.enable()?;
            }
        };

        expanded.into()
    } else {
        target_ty
            .span()
            .unwrap()
            .error("target type isn't a function type")
            .emit();
        TokenStream::new()
    }
}
