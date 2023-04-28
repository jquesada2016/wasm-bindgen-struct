#[macro_use]
extern crate proc_macro_error;

#[macro_use]
mod utils;
mod exts;
mod model;

use crate::model::Model;
use proc_macro_error::proc_macro_error;
use quote::ToTokens;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn wasm_bindgen_struct(
  _: proc_macro::TokenStream,
  input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
  let model = syn::parse_macro_input!(input as Model);

  model.to_token_stream().into()
}

#[cfg(test)]
mod macro_tests {
  use super::*;
  use crate::exts::TokenStreamExt;

  #[track_caller]
  fn parse_model(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let model = syn::parse2::<Model>(input).unwrap();

    model.to_token_stream()
  }

  #[track_caller]
  fn assert_eq_token_stream(
    output: proc_macro2::TokenStream,
    expected_output: proc_macro2::TokenStream,
  ) {
    let left = output.to_pretty();
    let right = expected_output.to_pretty();

    let theme = termdiff::SignsColorTheme::default();

    let diff = termdiff::DrawDiff::new(&right, &left, &theme);

    assert_eq!(left, right, "\n\n{diff}");
  }

  #[test]
  fn simple_struct() {
    let input = quote! {
      struct JsType {
        my_prop_1: String,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        type JsType;

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "myProp1")]
        fn my_prop_1(this: &JsType) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "myProp1")]
        fn set_my_prop_1(this: &JsType, value: String);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_with_field_rename() {
    let input = quote! {
      struct JsType {
        #[opts(js_name = "prop")]
        my_prop_1: String,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        type JsType;

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "prop")]
        fn my_prop_1(this: &JsType) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "prop")]
        fn set_my_prop_1(this: &JsType, value: String);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_on_other_type() {
    let input = quote! {
      #[opts(on = SomeType)]
      struct JsType {
        my_prop_1: String,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "myProp1")]
        fn my_prop_1(this: &SomeType) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "myProp1")]
        fn set_my_prop_1(this: &SomeType, value: String);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_final_and_structural() {
    let input = quote! {
      #[opts(final_)]
      struct JsType {
        my_prop_1: String,
        #[opts(structural)]
        prop: String,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        type JsType;

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "myProp1")]
        #[wasm_bindgen(final)]
        fn my_prop_1(this: &JsType) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "myProp1")]
        #[wasm_bindgen(final)]
        fn set_my_prop_1(this: &JsType, value: String);

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "prop")]
        fn prop(this: &JsType) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "prop")]
        fn set_prop(this: &JsType, value: String);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_only_get_set() {
    let input = quote! {
      struct JsType {
        #[opts(getter)]
        my_prop_1: String,
        #[opts(setter)]
        prop: String,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        type JsType;

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "myProp1")]
        fn my_prop_1(this: &JsType) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "prop")]
        fn set_prop(this: &JsType, value: String);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_global_setter_with_local_getter() {
    let input = quote! {
      #[opts(setter)]
      struct JsType {
        #[opts(getter)]
        my_prop_1: String,
        prop: String,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        type JsType;

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "myProp1")]
        fn my_prop_1(this: &JsType) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "myProp1")]
        fn set_my_prop_1(this: &JsType, value: String);

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "prop")]
        fn set_prop(this: &JsType, value: String);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_js_class() {
    let input = quote! {
      #[opts(js_name = "String")]
      struct JsString {
        prop: String,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        #[wasm_bindgen(js_name = "String")]
        type JsString;

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_class = "String")]
        #[wasm_bindgen(js_name = "prop")]
        fn prop(this: &JsString) -> String;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_class = "String")]
        #[wasm_bindgen(js_name = "prop")]
        fn set_prop(this: &JsString, value: String);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_extends() {
    let input = quote! {
      #[opts(js_name = "String")]
      #[opts(extends = Object)]
      struct JsString {}
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        #[wasm_bindgen(js_name = "String")]
        #[wasm_bindgen(extends = Object)]
        type JsString;
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn struct_can_use_self_ty() {
    let input = quote! {
      struct JsType {
        a: Self,
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      #[::wasm_bindgen::prelude::wasm_bindgen]
      extern "C" {
        type JsType;

        #[wasm_bindgen(method, getter)]
        #[wasm_bindgen(js_name = "a")]
        fn a(this: &JsType) -> JsType;

        #[wasm_bindgen(method, setter)]
        #[wasm_bindgen(js_name = "a")]
        fn set_a(this: &JsType, value: JsType);
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn simpl_impl() {
    let input = quote! {
      impl JsType {
        fn example(&self);
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      impl JsType {
        fn example(&self) {
          #[::wasm_bindgen::prelude::wasm_bindgen]
          extern "C" {
            #[wasm_bindgen(method)]
            #[wasm_bindgen(js_name = "example")]
            fn example_js(this: &JsType);
          }

          self.example_js()
        }
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn impl_static() {
    let input = quote! {
      impl JsType {
        fn example();
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      impl JsType {
        fn example() {
          #[::wasm_bindgen::prelude::wasm_bindgen]
          extern "C" {
            #[wasm_bindgen(static_method_of = JsType)]
            #[wasm_bindgen(js_name = "example")]
            fn example_js();
          }

          Self::example_js()
        }
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn impl_can_map_value() {
    let input = quote! {
      impl JsType {
        fn example(&self) -> MapValue<T, U>;
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      impl JsType {
        fn example(&self) -> U {
          #[::wasm_bindgen::prelude::wasm_bindgen]
          extern "C" {
            #[wasm_bindgen(method)]
            #[wasm_bindgen(js_name = "example")]
            fn example_js(this: &JsType) -> T;
          }

          self.example_js()
        }
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn impl_can_async_with_args_can_map_value() {
    let input = quote! {
      impl JsType {
        async fn example(&self, a: String) -> MapValue<T, U>;
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      impl JsType {
        async fn example(&self, a: String) -> U {
          #[::wasm_bindgen::prelude::wasm_bindgen]
          extern "C" {
            #[wasm_bindgen(method)]
            #[wasm_bindgen(js_name = "example")]
            async fn example_js(this: &JsType, a: String) -> T;
          }

          self.example_js(a)
        }
      }
    };

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn impl_can_async_with_args_can_map_value_with_block() {
    let input = quote! {
      impl JsType {
        async fn example(&self, a: String) -> MapValue<T, U> {
          self.example_js(a).into()
        }
      }
    };

    let output = parse_model(input);

    let expected_output = quote! {
      impl JsType {
        async fn example(&self, a: String) -> U {
          #[::wasm_bindgen::prelude::wasm_bindgen]
          extern "C" {
            #[wasm_bindgen(method)]
            #[wasm_bindgen(js_name = "example")]
            async fn example_js(this: &JsType, a: String) -> T;
          }

          self.example_js(a).into()
        }
      }
    };

    assert_eq_token_stream(output, expected_output);
  }
}
