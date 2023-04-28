use crate::exts::IdentExt;
use attribute_derive::Attribute;
use proc_macro2::TokenStream;
use quote::{
  quote_spanned,
  ToTokens,
};
use syn::{
  parse::Parse,
  parse_quote,
  parse_quote_spanned,
};

pub enum Model {
  Struct(Struct),
  Impl(Impl),
}

impl syn::parse::Parse for Model {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let this = if let Ok(item_struct) = syn::ItemStruct::parse(input) {
      Self::Struct(Struct::try_from(item_struct)?)
    } else if let Ok(item_impl) = syn::ItemImpl::parse(input) {
      Self::Impl(Impl::try_from(item_impl)?)
    } else {
      abort!(
        input.span(),
        "macro can only be used on a struct or impl block"
      );
    };

    Ok(this)
  }
}

impl quote::ToTokens for Model {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    match self {
      Model::Struct(item) => item.to_tokens(tokens),
      Model::Impl(impl_) => {
        impl_.to_tokens(tokens);
      }
    }
  }
}

#[derive(Debug)]
pub struct Struct {
  vis: syn::Visibility,
  name: syn::Ident,
  on: Option<syn::Type>,
  getters: GetterKind,
  final_: bool,
  js_name: Option<syn::Lit>,
  js_namespace: Vec<syn::Lit>,
  module: Option<syn::Lit>,
  raw_module: Option<syn::Lit>,
  extends: Option<syn::Type>,
  fields: Vec<Field>,
}

impl TryFrom<syn::ItemStruct> for Struct {
  type Error = syn::Error;

  fn try_from(item: syn::ItemStruct) -> Result<Self, Self::Error> {
    let StructAttributes {
      on,
      extends,
      getter,
      setter,
      final_: r#final,
      js_name,
      js_namespace,
      module,
      raw_module,
    } = StructAttributes::from_attributes(&item.attrs)?;

    if getter && setter {
      emit_warning!(
        item.ident,
        "`getter` and `setter` are implied by default, only set if you need \
         one or the other"
      );
    }

    Ok(Self {
      vis: item.vis,
      name: item.ident,
      on,
      extends,
      getters: GetterKind::new(getter, setter),
      final_: r#final,
      js_name,
      js_namespace,
      module,
      raw_module,
      fields: item
        .fields
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, _>>()?,
    })
  }
}

impl quote::ToTokens for Struct {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    let extern_type = self.extern_type();

    let fields = self
      .fields
      .iter()
      .map(|field| field.to_tokens_with_global(self))
      .collect::<TokenStream>();

    let module = self
      .module
      .as_ref()
      .map(|module| quote! { (module = #module) })
      .or_else(|| {
        self
          .raw_module
          .as_ref()
          .map(|raw_module| quote! { (raw_module = #raw_module) })
      });

    let output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen #module]
      extern "C" {
        #extern_type

        #fields
      }
    };

    tokens.extend(output);
  }
}

impl Struct {
  fn extern_type(&self) -> TokenStream {
    let Self {
      vis,
      name,
      on,
      extends,
      getters: _,
      final_: _,
      js_name,
      js_namespace: _,
      module: _,
      raw_module: _,
      fields: _,
    } = self;

    if on.is_some() {
      return quote! {};
    }

    let js_name = js_name
      .as_ref()
      .map(|js_name| quote! { #[wasm_bindgen(js_name = #js_name)] });

    let extends = extends
      .as_ref()
      .map(|extends| quote! { #[wasm_bindgen(extends = #extends)] });

    quote! {
      #js_name
      #extends
      #vis type #name;
    }
  }
}

pub struct Impl {
  ty: syn::Type,
  options: ImplAttributes,
  items: Vec<Method>,
}

impl TryFrom<syn::ItemImpl> for Impl {
  type Error = syn::Error;

  fn try_from(item: syn::ItemImpl) -> Result<Self, Self::Error> {
    Ok(Self {
      options: ImplAttributes::from_attributes(&item.attrs)?,
      ty: *item.self_ty.clone(),
      items: Method::try_from_impl(item)?,
    })
  }
}

impl ToTokens for Impl {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Self { ty, options, items } = self;

    let output = items
      .iter()
      .map(|item| item.to_tokens_with_global(ty, options));

    let output = quote! {
      impl #ty {
        #(#output)*
      }
    };

    tokens.extend(output);
  }
}

#[derive(Debug)]
struct Field {
  vis: syn::Visibility,
  name: syn::Ident,
  final_: bool,
  structural: bool,
  js_name: Option<syn::Lit>,
  getters: GetterKind,
  ty: syn::Type,
}

impl TryFrom<syn::Field> for Field {
  type Error = syn::Error;

  fn try_from(field: syn::Field) -> Result<Self, Self::Error> {
    let FieldAttributes {
      getter,
      setter,
      final_: r#final,
      structural,
      js_name,
      r#static,
    } = FieldAttributes::from_attributes(&field.attrs)?;

    Ok(Self {
      vis: field.vis,
      name: field
        .ident
        .unwrap_or_else(|| abort_call_site!("tuple structs are not allowed")),
      final_: r#final,
      structural,
      js_name,
      getters: GetterKind::new(getter, setter),
      ty: field.ty,
    })
  }
}

impl Field {
  fn to_tokens_with_global(&self, global: &Struct) -> TokenStream {
    let Struct {
      name,
      on,
      getters: getters_global,
      final_: final_global,
      js_name: js_class,
      js_namespace,
      vis: _,
      module: _,
      raw_module: _,
      extends: _,
      fields: _,
    } = global;

    let ty_name = on
      .as_ref()
      .map(|on| quote! { #on })
      .unwrap_or_else(|| quote! { #name });

    let Self {
      vis,
      name,
      final_,
      structural,
      js_name,
      getters: get_kind,
      ty,
    } = self;

    let ty = {
      let mut ty = ty.clone();

      if is_self_ty(&ty) {
        ty = syn::parse2(ty_name.to_token_stream()).unwrap();
      }

      ty
    };

    let final_ = (!*structural && (*final_global || *final_))
      .then(|| quote! { #[wasm_bindgen(final)] });

    let js_class = js_class
      .as_ref()
      .map(|js_class| quote! { #[wasm_bindgen(js_class = #js_class)] });

    let js_name = js_name
      .as_ref()
      .map(|js_name| quote! { #[wasm_bindgen(js_name = #js_name)] })
      .unwrap_or_else(|| {
        let ident = name.to_camel_from_snake();

        let ident_name = ident.to_string();

        quote_spanned! { ident.span() => #[wasm_bindgen(js_name = #ident_name)] }
      });

    let js_namespace = (!js_namespace.is_empty())
      .then(|| quote! { #[wasm_bindgen(js_namespace = [#(#js_namespace),*])] });

    let getter_fn = apply_getter_rules(*getters_global, *get_kind)
      .is_getter()
      .then(|| {
        quote! {
          #[wasm_bindgen(method, getter)]
          #js_class
          #js_name
          #js_namespace
          #final_
          #vis fn #name(this: &#ty_name) -> #ty;
        }
      });

    let setter_fn = apply_getter_rules(*getters_global, *get_kind)
      .is_setter()
      .then(|| {
        let set_name = quote::format_ident!("set_{name}");

        quote! {
          #[wasm_bindgen(method, setter)]
          #js_class
          #js_name
          #js_namespace
          #final_
          #vis fn #set_name(this: &#ty_name, value: #ty);
        }
      });

    quote! {
      #getter_fn

      #setter_fn
    }
  }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
enum GetterKind {
  #[default]
  None,
  Get,
  Set,
  Both,
}

impl GetterKind {
  fn new(getter: bool, setter: bool) -> Self {
    if !(getter || setter) {
      Self::default()
    } else if getter && setter {
      Self::Both
    } else if getter {
      Self::Get
    } else {
      Self::Set
    }
  }

  fn merge(self, other: Self) -> Self {
    let is_getter = self.is_getter() || other.is_getter();

    let is_setter = self.is_setter() || other.is_setter();

    Self::new(is_getter, is_setter)
  }

  fn is_getter(self) -> bool {
    matches!(self, Self::Both | Self::Get)
  }

  fn is_setter(self) -> bool {
    matches!(self, Self::Both | Self::Set)
  }

  fn is_none(self) -> bool {
    self == Self::None
  }
}

struct Method {
  vis: syn::Visibility,
  sig: syn::Signature,
  body: Option<syn::Block>,
  constructor: bool,
  final_: bool,
  structural: bool,
  js_name: Option<syn::Lit>,
  getter: bool,
  setter: bool,
  indexing_getter: bool,
  indexing_setter: bool,
  indexing_deleter: bool,
  variadic: bool,
}

impl TryFrom<syn::TraitItemFn> for Method {
  type Error = syn::Error;

  fn try_from(f: syn::TraitItemFn) -> Result<Self, Self::Error> {
    let MethodAttributes {
      pub_,
      constructor,
      final_,
      getter,
      setter,
      structural,
      indexing_getter,
      indexing_setter,
      indexing_deleter,
      js_name,
      variadic,
    } = MethodAttributes::from_attributes(&f.attrs)?;

    if !f.sig.generics.params.is_empty() {
      abort!(f.sig.generics, "generics on methods are not supported");
    }

    if f.sig.constness.is_some() {
      abort!(f.sig, "methods cannot be const");
    }

    if f.sig.abi.is_some() {
      abort!(f.sig, "abi cannot be specified");
    }

    if f.sig.unsafety.is_some() {
      abort!(f.sig, "methods cannot be declared unsafe");
    }

    Ok(Self {
      vis: pub_
        .then(|| parse_quote! { pub })
        .unwrap_or_else(|| parse_quote! {}),
      sig: f.sig,
      body: f.default,
      constructor,
      final_,
      structural,
      js_name,
      getter,
      setter,
      indexing_getter,
      indexing_setter,
      indexing_deleter,
      variadic,
    })
  }
}

impl Method {
  fn try_from_impl(item: syn::ItemImpl) -> Result<Vec<Self>, syn::Error> {
    let impl_attrs = ImplAttributes::from_attributes(&item.attrs)?;

    if !item.generics.params.is_empty() {
      abort!(item.generics, "generics are not supported");
    }

    // Reinterpret impl items as trait items, because trait
    // items allow for an optional default block
    let methods = item
      .items
      .into_iter()
      .map(|item| item.to_token_stream())
      .map(syn::parse2::<syn::TraitItemFn>)
      .collect::<Result<Vec<_>, _>>()
      .map_err(|err| {
        abort_call_site!("only methods are allowed");
      })
      .unwrap()
      .into_iter()
      .map(TryFrom::try_from)
      .collect::<Result<Vec<_>, _>>()?;

    Ok(methods)
  }

  fn to_tokens_with_global(
    &self,
    ty: &syn::Type,
    options: &ImplAttributes,
  ) -> TokenStream {
    let ImplAttributes {
      final_: final_global,
      js_name: js_class,
      js_namespace,
      module: _,
      raw_module: _,
    } = options;

    let Self {
      vis,
      sig,
      body,
      constructor,
      final_,
      structural,
      js_name,
      getter,
      setter,
      indexing_getter,
      indexing_setter,
      indexing_deleter,
      variadic,
    } = self;

    let static_opt = self
      .is_static()
      .then(|| quote! { #[wasm_bindgen(static_method_of = #ty)] });

    let method =
      (!self.is_static()).then(|| quote! { #[wasm_bindgen(method)] });

    let constructor =
      constructor.then(|| quote! { #[wasm_bindgen(constructor)] });

    let final_ = ((*final_global && !structural) || *final_)
      .then(|| quote! { #[wasm_bindgen(final)] });

    let js_class = js_class
      .as_ref()
      .map(|js_class| quote! { 3[wasm_bindgen(js_class = #js_class)] });

    let js_name = js_name
      .as_ref()
      .map(|js_name| quote! { #[wasm_bindgen(js_name = #js_name)] })
      .unwrap_or_else(|| {
        let ident = sig.ident.to_camel_from_snake().to_string();

        quote! { #[wasm_bindgen(js_name = #ident)] }
      });

    let js_namespace = (!js_namespace.is_empty()).then(|| {
      let parts = js_namespace.iter().map(|part| quote! { #part });

      quote! { #[wasm_bindgen(js_namespace = [#(#parts),*])] }
    });

    let getter = getter.then(|| quote! { #[wasm_bindgen(getter)] });
    let setter = setter.then(|| quote! { #[wasm_bindgen(setter)] });

    let indexing_getter =
      indexing_getter.then(|| quote! { #[wasm_bindgen(indexing_getter)] });

    let indexing_setter =
      indexing_setter.then(|| quote! { #[wasm_bindgen(indexing_setter)] });

    let indexing_deleter =
      indexing_deleter.then(|| quote! { #[wasm_bindgen(indexing_deleter)] });

    let variadic = variadic.then(|| quote! { #[wasm_bindgen(variadic)] });

    let catch = is_result_from_return_ty(&sig.output)
      .then(|| quote! { #[wasm_bindgen(catch)] });

    let outer_sig = {
      let mut sig = sig.clone();

      sig.output = self.outer_return_ty();

      sig
    };

    let sig = {
      let mut sig = sig.clone();

      // Rename method to have a trailing `_js`
      sig.ident = quote::format_ident!("{}_js", sig.ident);

      // Remove receiver from the inputs list and replace with
      // `this: &ty`
      if let Some(receiver) = sig.receiver() {
        let ref_ = receiver.reference.is_some().then(|| quote! { & });

        sig.inputs[0] = parse_quote! { this: #ref_ #ty };
      };

      // Set the output type to match the `MapValue` if needed
      sig.output = self.inner_return_ty();

      // Replace `Self` return type with the real name of the type
      if is_self_ty_from_return_ty(&sig.output) {
        sig.output = parse_quote! { -> #ty };
      }

      sig
    };

    let body = self.body();

    quote! {
      #outer_sig {
        #[::wasm_bindgen::prelude::wasm_bindgen]
        extern "C" {
          #static_opt
          #method
          #constructor
          #final_
          #js_class
          #js_name
          #getter
          #setter
          #js_namespace
          #indexing_getter
          #indexing_setter
          #indexing_deleter
          #variadic
          #catch
          #vis #sig;
        }

        #body
      }
    }
  }

  fn is_static(&self) -> bool {
    self.sig.receiver().is_none()
  }

  fn body(&self) -> TokenStream {
    fn fn_arg_to_ident(arg: &syn::FnArg) -> Option<syn::Ident> {
      if let syn::FnArg::Typed(syn::PatType { pat, .. }) = arg {
        if let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &**pat {
          Some(ident.clone())
        } else {
          abort!(pat, "only idents can be used here");
        }
      } else {
        None
      }
    }

    self
      .body
      .as_ref()
      .map(|body| {
        let stmts = &body.stmts;

        quote! { #(#stmts)* }
      })
      .unwrap_or_else(|| {
        let fn_name = quote::format_ident!("{}_js", self.sig.ident);

        if self.sig.receiver().is_some() {
          let inputs = self
            .sig
            .inputs
            .clone()
            .into_iter()
            .skip(1)
            .map(|arg| fn_arg_to_ident(&arg))
            .collect::<syn::punctuated::Punctuated<_, syn::Token![,]>>();

          quote! { self.#fn_name(#inputs) }
        } else {
          let inputs = &self
            .sig
            .inputs
            .iter()
            .map(fn_arg_to_ident)
            .collect::<syn::punctuated::Punctuated<_, syn::Token![,]>>(
          );

          quote! { Self::#fn_name(#inputs) }
        }
      })
  }

  fn map_value_types(&self) -> Option<(syn::Type, syn::Type)> {
    if let syn::ReturnType::Type(_, ty) = &self.sig.output {
      if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
      }) = &**ty
      {
        let map_value_ident: syn::Ident = parse_quote! { MapValue };

        if segments.len() != 1 || segments[0].ident != map_value_ident {
          return None;
        }

        if let syn::PathArguments::AngleBracketed(
          syn::AngleBracketedGenericArguments { args, .. },
        ) = &segments[0].arguments
        {
          if args.len() != 2 {
            abort!(args, "`MapValue` must have exactly 2 type arguments");
          }

          let types = args
            .iter()
            .map(|arg| {
              if let syn::GenericArgument::Type(ty) = arg {
                ty
              } else {
                abort!(arg, "only types withn `MapValue` are supported")
              }
            })
            .collect::<Vec<_>>();

          Some((types[0].clone(), types[1].clone()))
        } else {
          None
        }
      } else {
        None
      }
    } else {
      None
    }
  }

  fn inner_return_ty(&self) -> syn::ReturnType {
    let mut return_ty = self.sig.output.clone();

    if let Some((inner, _)) = self.map_value_types() {
      if let syn::ReturnType::Type(_, ty) = &mut return_ty {
        **ty = inner;
      }
    }

    return_ty
  }

  fn outer_return_ty(&self) -> syn::ReturnType {
    let mut return_ty = self.sig.output.clone();

    if let Some((_, outer)) = self.map_value_types() {
      if let syn::ReturnType::Type(_, ty) = &mut return_ty {
        **ty = outer;
      }
    }

    return_ty
  }
}

#[derive(Attribute)]
#[attribute(ident = opts)]
struct StructAttributes {
  #[attribute(conflicts = [extends])]
  on: Option<syn::Type>,
  #[attribute(conflicts = [on])]
  extends: Option<syn::Type>,
  getter: bool,
  setter: bool,
  final_: bool,
  js_name: Option<syn::Lit>,
  #[attribute(optional)]
  js_namespace: Vec<syn::Lit>,
  #[attribute(conflicts = [raw_module])]
  module: Option<syn::Lit>,
  #[attribute(conflicts = [module])]
  raw_module: Option<syn::Lit>,
}

#[derive(Debug, Attribute)]
#[attribute(ident = opts)]
struct FieldAttributes {
  getter: bool,
  setter: bool,
  final_: bool,
  structural: bool,
  js_name: Option<syn::Lit>,
  r#static: bool,
}

#[derive(Debug, Attribute)]
#[attribute(ident = opts)]
struct ImplAttributes {
  final_: bool,
  js_name: Option<syn::Lit>,
  #[attribute(optional)]
  js_namespace: Vec<syn::Lit>,
  #[attribute(conflicts = [raw_module])]
  module: Option<syn::Lit>,
  #[attribute(conflicts = [module])]
  raw_module: Option<syn::Lit>,
}

#[derive(Debug, Attribute)]
#[attribute(ident = opts)]
struct MethodAttributes {
  pub_: bool,
  constructor: bool,
  final_: bool,
  getter: bool,
  setter: bool,
  structural: bool,
  indexing_getter: bool,
  indexing_setter: bool,
  indexing_deleter: bool,
  js_name: Option<syn::Lit>,
  variadic: bool,
}

// #[derive(Attribute)]
// #[attribute(ident = opts)]
// struct Attributes {
//   on: Option<syn::Type>,
//   catch: bool,
//   constructor: bool,
//   extends: Option<syn::Type >,
//   getter: bool,
//   setter: bool,
//   r#final: bool,
//   indexing_getter: bool,
//   indexing_setter: bool,
//   indexing_deleter: bool,
//   js_class: Option<syn::Lit>,
//   js_name: Option<syn::Lit>,
//   #[attribute(optional)]
//   js_namespace: Vec<syn::Lit>,
//   method: bool,
//   module: Option<syn::Lit>,
//   raw_module: Option<syn::Lit>,
//   static_method_of: Option<syn::Lit>,
//   structural: bool,
//   variadic: bool,
//   vendor_prefix: bool,
// }

fn is_result_from_return_ty(return_ty: &syn::ReturnType) -> bool {
  if let syn::ReturnType::Type(_, ty) = return_ty {
    is_result(ty)
  } else {
    false
  }
}

fn is_result(ty: &syn::Type) -> bool {
  if let syn::Type::Path(syn::TypePath {
    path: syn::Path { segments, .. },
    ..
  }) = ty
  {
    let result_ident: syn::Ident = syn::parse_quote!(Result);

    segments.first().unwrap().ident == result_ident
  } else {
    false
  }
}

fn apply_getter_rules(global: GetterKind, local: GetterKind) -> GetterKind {
  if global.is_none() && local.is_none() {
    GetterKind::Both
  } else if global.is_none() {
    local
  } else {
    global.merge(local)
  }
}

fn is_self_ty_from_return_ty(return_ty: &syn::ReturnType) -> bool {
  if let syn::ReturnType::Type(_, ty) = return_ty {
    is_self_ty(ty)
  } else {
    false
  }
}

fn is_self_ty(ty: &syn::Type) -> bool {
  let self_ty: syn::Type = parse_quote!(Self);

  self_ty == *ty
}
