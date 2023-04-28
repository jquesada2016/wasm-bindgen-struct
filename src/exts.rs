use itertools::Itertools;

pub trait IdentExt {
  fn to_camel_from_snake(&self) -> Self;
}

impl IdentExt for syn::Ident {
  fn to_camel_from_snake(&self) -> Self {
    let span = self.span();

    let ident = self.to_string();

    let mut ident_name = String::with_capacity(ident.len());

    let mut prev_char_is_underscore = false;

    for c in ident.chars() {
      if c == '_' {
        prev_char_is_underscore = true;
        continue;
      } else if prev_char_is_underscore && c.is_alphabetic() {
        ident_name.extend(c.to_uppercase());
      } else {
        ident_name.push(c);
      }

      prev_char_is_underscore = false;
    }

    syn::Ident::new(&ident_name, span)
  }
}

#[cfg(test)]
mod ident_to_camel_from_snake {
  use super::*;

  #[test]
  fn simple() {
    let ident: syn::Ident = syn::parse_quote!(a_little_test);

    assert_eq!(ident.to_camel_from_snake().to_string(), "aLittleTest");
  }

  #[test]
  fn multiple_underscores() {
    let ident: syn::Ident = syn::parse_quote!(a__little_test);

    assert_eq!(ident.to_camel_from_snake().to_string(), "aLittleTest");
  }

  #[test]
  fn with_numbers() {
    let ident: syn::Ident = syn::parse_quote!(a_2_little_test);

    assert_eq!(ident.to_camel_from_snake().to_string(), "a2LittleTest");
  }

  #[test]
  fn with_numbers_as_start() {
    let ident: syn::Ident = syn::parse_quote!(a_2little_test);

    assert_eq!(ident.to_camel_from_snake().to_string(), "a2littleTest");
  }
}

#[cfg(test)]
pub trait TokenStreamExt {
  fn to_pretty(&self) -> String;
}

#[cfg(test)]
impl TokenStreamExt for proc_macro2::TokenStream {
  #[track_caller]
  fn to_pretty(&self) -> String {
    let file = syn::parse2::<syn::File>(self.clone())
      .expect("failed to parse `TokenStream` as `syn::File`");

    prettyplease::unparse(&file)
  }
}
