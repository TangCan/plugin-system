//! `plugctx-derive` — 可选过程宏，减少 `Plugin` 样板（FR27）。
//!
//! 核心 crate `plugctx` **不**依赖本 crate；插件作者按需引入。

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Error as SynError, Path, Result as SynResult,
    Token,
};

/// 从结构体上的 `#[plugin(depends(A, B))]` 生成 [`plugctx::Plugin`] 实现。
///
/// - `dependencies()`：返回声明类型的 `TypeId` 列表（无 `depends` 则为空）。
/// - `build()`：委托到用户固有方法 `fn on_build(&self, ctx: &mut Context) -> Result<(), Error>`。
///
/// # 示例
///
/// ```ignore
/// use plugctx_derive::Plugin;
///
/// #[derive(Plugin)]
/// #[plugin(depends(Logger))]
/// struct MyPlugin;
///
/// impl MyPlugin {
///     fn on_build(&self, ctx: &mut plugctx::Context) -> Result<(), plugctx::Error> {
///         let _ = ctx.get::<Logger>();
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_derive(Plugin, attributes(plugin))]
pub fn derive_plugin(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand_plugin(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand_plugin(input: DeriveInput) -> SynResult<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(_) => {}
        Data::Enum(_) | Data::Union(_) => {
            return Err(SynError::new_spanned(
                &input.ident,
                "#[derive(Plugin)] 仅支持结构体（struct）",
            ));
        }
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let depends = parse_depends_attrs(&input.attrs)?;

    let depend_type_ids = depends.iter().map(|path| {
        quote! {
            ::std::any::TypeId::of::<#path>()
        }
    });

    Ok(quote! {
        impl #impl_generics ::plugctx::Plugin for #name #ty_generics #where_clause {
            fn dependencies(&self) -> ::std::vec::Vec<::std::any::TypeId> {
                ::std::vec![
                    #(#depend_type_ids),*
                ]
            }

            fn build(
                &self,
                ctx: &mut ::plugctx::Context,
            ) -> ::std::result::Result<(), ::plugctx::Error> {
                Self::on_build(self, ctx)
            }
        }
    })
}

struct DependsList {
    types: Punctuated<Path, Token![,]>,
}

impl Parse for DependsList {
    fn parse(input: syn::parse::ParseStream) -> SynResult<Self> {
        let content;
        syn::parenthesized!(content in input);
        let types = Punctuated::parse_terminated(&content)?;
        Ok(Self { types })
    }
}

fn parse_depends_attrs(attrs: &[Attribute]) -> SynResult<Vec<Path>> {
    let mut deps = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("plugin") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("depends") {
                let list: DependsList = meta.input.parse()?;
                deps.extend(list.types);
                Ok(())
            } else {
                Err(meta.error("不支持的 #[plugin(...)] 键；仅支持 depends(...)"))
            }
        })?;
    }
    Ok(deps)
}
