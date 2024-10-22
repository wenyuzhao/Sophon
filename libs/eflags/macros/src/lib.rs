use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn eflags(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse representation type
    let attr: TokenStream = (!attr.is_empty())
        .then_some(attr)
        .unwrap_or_else(|| quote!(usize).into());
    let ty = syn::parse_macro_input!(attr as syn::Type);
    // Parse enum name and visibility
    let item = syn::parse_macro_input!(item as syn::ItemEnum);
    let name = item.clone().ident;
    // Parse variants
    let mut variants = Vec::with_capacity(item.variants.len());
    let mut names = Vec::with_capacity(item.variants.len());
    let mut all_values = Vec::with_capacity(item.variants.len());
    for variant in &item.variants {
        let variant_name = variant.ident.clone();
        // TODO: default value?
        let expr = variant.discriminant.clone().unwrap().1;
        variants.push(quote!(pub const #variant_name: #name = Self { value: #expr };));
        names.push(quote!(stringify!(#variant_name)));
        all_values.push(quote!(Self::#variant_name));
    }

    quote! {
        #[derive(Clone, Copy, Hash, PartialEq, Eq)]
        #item

        impl ::eflags::Flag for #name {
            type Value = #ty;

            fn from_raw(value: Self::Value) -> Self {
                unsafe {
                    ::core::mem::transmute(value)
                }
            }

            fn name(&self) -> &'static str {
                match self {
                    #(#all_values => #names,)*
                }
            }

            fn value(&self) -> Self::Value {
                unsafe {
                    ::core::mem::transmute(*self)
                }
            }

            fn values() -> &'static [Self] {
                &[#(#all_values,)*]
            }
        }

        impl core::ops::Not for #name {
            type Output = ::eflags::FlagSet<#name>;
            #[inline(always)]
            fn not(self) -> Self::Output {
                use ::eflags::Flag;
                ::eflags::FlagSet::from_raw(!self.value())
            }
        }

        impl core::ops::BitAnd<#name> for #name {
            type Output = ::eflags::FlagSet<Self>;
            #[inline(always)]
            fn bitand(self, x: Self) -> Self::Output {
                use ::eflags::Flag;
                ::eflags::FlagSet::from_raw(self.value() & x.value())
            }
        }

        impl core::ops::BitAnd<::eflags::FlagSet<#name>> for #name {
            type Output = ::eflags::FlagSet<Self>;
            #[inline(always)]
            fn bitand(self, x: ::eflags::FlagSet<Self>) -> Self::Output {
                use ::eflags::Flag;
                ::eflags::FlagSet::from_raw(self.value() & x.value())
            }
        }

        impl core::ops::BitOr<#name> for #name {
            type Output = ::eflags::FlagSet<Self>;
            #[inline(always)]
            fn bitor(self, x: Self) -> Self::Output {
                use ::eflags::Flag;
                ::eflags::FlagSet::from_raw(self.value() | x.value())
            }
        }

        impl core::ops::BitOr<::eflags::FlagSet<Self>> for #name {
            type Output = ::eflags::FlagSet<Self>;
            #[inline(always)]
            fn bitor(self, x: ::eflags::FlagSet<Self>) -> Self::Output {
                use ::eflags::Flag;
                ::eflags::FlagSet::from_raw(self.value() | x.value())
            }
        }

        impl core::ops::BitXor<#name> for #name {
            type Output = ::eflags::FlagSet<Self>;
            #[inline(always)]
            fn bitxor(self, x: Self) -> Self::Output {
                use ::eflags::Flag;
                ::eflags::FlagSet::from_raw(self.value() ^ x.value())
            }
        }

        impl core::ops::BitXor<::eflags::FlagSet<Self>> for #name {
            type Output = ::eflags::FlagSet<Self>;
            #[inline(always)]
            fn bitxor(self, x: ::eflags::FlagSet<Self>) -> Self::Output {
                use ::eflags::Flag;
                ::eflags::FlagSet::from_raw(self.value() ^ x.value())
            }
        }

        impl From<#ty> for #name {
            #[inline(always)]
            fn from(value: #ty) -> Self {
                use ::eflags::Flag;
                Self::from_raw(value)
            }
        }

        impl From<#name> for #ty {
            #[inline(always)]
            fn from(x: #name) -> #ty {
                use ::eflags::Flag;
                x.value()
            }
        }

        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                use ::eflags::Flag;
                write!(f, "{}", self.name())
            }
        }

        impl core::fmt::Binary for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                use ::eflags::Flag;
                core::fmt::Binary::fmt(&self.value(), f)
            }
        }

        impl core::fmt::LowerHex for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                use ::eflags::Flag;
                core::fmt::LowerHex::fmt(&self.value(), f)
            }
        }

        impl core::fmt::UpperHex for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                use ::eflags::Flag;
                core::fmt::UpperHex::fmt(&self.value(), f)
            }
        }
    }
    .into()
}
