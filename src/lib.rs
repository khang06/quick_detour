#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote_spanned};
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
                    format_ident!("__arg_{}", i, span = Span::mixed_site()),
                    <Token![:]>::default(),
                )),
                ty: x.ty.clone(),
            }),
        );
        let detour_arg_types = fn_ty.inputs.iter().map(|x| &x.ty);
        let detour_ffi_args = Punctuated::<Ident, Token![,]>::from_iter(
            (0..fn_ty.inputs.len())
                .map(|i| format_ident!("__arg_{}", i, span = Span::mixed_site())),
        );

        let expanded = quote_spanned!(Span::mixed_site() => {
            #[allow(non_standard_style)]
            static __hook: ::detour::StaticDetour<#fn_ty> = {
                #[inline(never)]
                #[allow(unused_unsafe)]

                #detour_unsafe #detour_abi fn __ffi_detour(#detour_args) #detour_ret {
                    #[allow(unused_unsafe)]
                    (__hook.__detour())(#detour_ffi_args)
                }
                ::detour::StaticDetour::__new(__ffi_detour)
            };
            // this hacky match statement is required to get the compiler to actually be able to assume types
            match ({
                #[allow(non_standard_style)]
                fn __type_funnel<__F>(f: __F) -> __F
                where
                    __F: ::core::ops::FnOnce(&::detour::StaticDetour<#fn_ty>, #(#detour_arg_types ,)*) #detour_ret
                {
                    f
                }

                __type_funnel
            })(#custom) {
                custom => __hook.initialize(#target, move |#detour_args| {
                    custom(&__hook, #detour_ffi_args)
                })?.enable()?,
            }
        });

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
