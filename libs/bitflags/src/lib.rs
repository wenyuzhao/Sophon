use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum};

#[proc_macro_attribute]
pub fn bitflags(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse representation type
    let attr: TokenStream = (!attr.is_empty())
        .then_some(attr)
        .unwrap_or_else(|| quote!(usize).into());
    let ty = parse_macro_input!(attr as syn::Type);
    // Parse enum name and visibility
    let item = parse_macro_input!(item as ItemEnum);
    let vis = item.vis.clone();
    let name = item.ident;
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
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #vis struct #name {
            pub value: #ty,
        }

        #[allow(non_upper_case_globals)]
        impl #name {
            #(#variants)*

            pub const fn names() -> &'static [&'static str] {
                &[#(#names,)*]
            }

            pub const fn values() -> &'static [Self] {
                &[#(#all_values,)*]
            }

            #[inline(always)]
            pub const fn contains(&self, flags: Self) -> bool {
                (self.value & flags.value) == flags.value
            }
        }

        impl core::ops::Not for #name {
            type Output = Self;
            #[inline(always)]
            fn not(self) -> Self::Output {
                Self { value: !self.value }
            }
        }

        impl core::ops::BitAnd for #name {
            type Output = Self;
            #[inline(always)]
            fn bitand(self, x: Self) -> Self::Output {
                Self { value: self.value & x.value }
            }
        }

        impl core::ops::BitAndAssign for #name {
            #[inline(always)]
            fn bitand_assign(&mut self, x: Self) {
                self.value &= x.value;
            }
        }

        impl core::ops::BitOr for #name {
            type Output = Self;
            #[inline(always)]
            fn bitor(self, x: Self) -> Self::Output {
                Self { value: self.value | x.value }
            }
        }

        impl core::ops::BitOrAssign for #name {
            #[inline(always)]
            fn bitor_assign(&mut self, x: Self) {
                self.value |= x.value;
            }
        }

        impl core::ops::BitXor for #name {
            type Output = Self;
            #[inline(always)]
            fn bitxor(self, x: Self) -> Self::Output {
                Self { value: self.value ^ x.value }
            }
        }

        impl core::ops::BitXorAssign for #name {
            #[inline(always)]
            fn bitxor_assign(&mut self, x: Self) {
                self.value ^= x.value;
            }
        }

        impl From<#ty> for #name {
            #[inline(always)]
            fn from(value: #ty) -> Self {
                Self { value }
            }
        }

        impl From<#name> for #ty {
            #[inline(always)]
            fn from(x: #name) -> #ty {
                x.value
            }
        }

        impl core::fmt::Debug for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let names = Self::names();
                let values = Self::values();
                let mut first = true;
                for i in 0..names.len() {
                    if self.contains(values[i]) {
                        if first {
                            first = false;
                        } else {
                            write!(f, " | ")?;
                        }
                        write!(f, "{}", names[i])?;
                    }
                }
                if first {
                    write!(f, "0")?;
                }
                Ok(())
            }
        }

        impl core::fmt::Binary for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Binary::fmt(&self.value, f)
            }
        }

        impl core::fmt::LowerHex for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerHex::fmt(&self.value, f)
            }
        }

        impl core::fmt::UpperHex for #name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperHex::fmt(&self.value, f)
            }
        }
    }
    .into()
}
