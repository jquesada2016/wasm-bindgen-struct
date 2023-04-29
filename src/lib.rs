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

  fn test_macro(
    input: proc_macro2::TokenStream,
    expected_output: proc_macro2::TokenStream,
  ) {
    let output = parse_model(input);

    assert_eq_token_stream(output, expected_output);
  }

  #[test]
  fn simple_struct() {
    test_macro(
      quote! {
        struct JsType {
          my_prop_1: String,
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn struct_with_field_rename() {
    test_macro(
      quote! {
        struct JsType {
          #[opts(js_name = "prop")]
          my_prop_1: String,
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn struct_on_other_type() {
    test_macro(
      quote! {
        #[opts(on = SomeType)]
        struct JsType {
          my_prop_1: String,
        }
      },
      quote! {
        #[::wasm_bindgen::prelude::wasm_bindgen]
        extern "C" {
          #[wasm_bindgen(method, getter)]
          #[wasm_bindgen(js_name = "myProp1")]
          fn my_prop_1(this: &SomeType) -> String;

          #[wasm_bindgen(method, setter)]
          #[wasm_bindgen(js_name = "myProp1")]
          fn set_my_prop_1(this: &SomeType, value: String);
        }
      },
    );
  }

  #[test]
  fn struct_final_and_structural() {
    test_macro(
      quote! {
        #[opts(final_)]
        struct JsType {
          my_prop_1: String,
          #[opts(structural)]
          prop: String,
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn struct_only_get_set() {
    test_macro(
      quote! {
        struct JsType {
          #[opts(getter)]
          my_prop_1: String,
          #[opts(setter)]
          prop: String,
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn struct_global_setter_with_local_getter() {
    test_macro(
      quote! {
        #[opts(setter)]
        struct JsType {
          #[opts(getter)]
          my_prop_1: String,
          prop: String,
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn struct_js_class() {
    test_macro(
      quote! {
        #[opts(js_name = "String")]
        struct JsString {
          prop: String,
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn struct_extends() {
    test_macro(
      quote! {
        #[opts(js_name = "String")]
        #[opts(extends = Object)]
        struct JsString {}
      },
      quote! {
        #[::wasm_bindgen::prelude::wasm_bindgen]
        extern "C" {
          #[wasm_bindgen(js_name = "String")]
          #[wasm_bindgen(extends = Object)]
          type JsString;
        }
      },
    );
  }

  #[test]
  fn struct_can_use_self_ty() {
    test_macro(
      quote! {
        struct JsType {
          a: Self,
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn simpl_impl() {
    test_macro(
      quote! {
        impl JsType {
          fn example(&self);
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn impl_static() {
    test_macro(
      quote! {
        impl JsType {
          fn example();
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn impl_can_map_value() {
    test_macro(
      quote! {
        impl JsType {
          fn example(&self) -> MapValue<T, U>;
        }
      },
      quote! {
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
      },
    );
  }

  #[test]
  fn impl_can_async_with_args_can_map_value() {
    test_macro(
      quote! {
        impl JsType {
          async fn example(&self, a: String) -> MapValue<T, U>;
        }
      },
      quote! {
        impl JsType {
          async fn example(&self, a: String) -> U {
            #[::wasm_bindgen::prelude::wasm_bindgen]
            extern "C" {
              #[wasm_bindgen(method)]
              #[wasm_bindgen(js_name = "example")]
              async fn example_js(this: &JsType, a: String) -> T;
            }

            self.example_js(a).await
          }
        }
      },
    );
  }

  #[test]
  fn impl_can_async_with_args_can_map_value_with_block() {
    test_macro(
      quote! {
        impl JsType {
          async fn example(&self, a: String) -> MapValue<T, U> {
            self.example_js(a).await.into()
          }
        }
      },
      quote! {
        impl JsType {
          async fn example(&self, a: String) -> U {
            #[::wasm_bindgen::prelude::wasm_bindgen]
            extern "C" {
              #[wasm_bindgen(method)]
              #[wasm_bindgen(js_name = "example")]
              async fn example_js(this: &JsType, a: String) -> T;
            }

            self.example_js(a).await.into()
          }
        }
      },
    );
  }

  #[test]
  fn impl_with_result_catches() {
    test_macro(
      quote! {
        impl JsType {
          fn example(&self) -> Result<String, JsValue>;
        }
      },
      quote! {
        impl JsType {
          fn example(&self) -> Result<String, JsValue> {
            #[::wasm_bindgen::prelude::wasm_bindgen]
            extern "C" {
              #[wasm_bindgen(method)]
              #[wasm_bindgen(js_name = "example")]
              #[wasm_bindgen(catch)]
              fn example_js(this: &JsType) -> Result<String, JsValue>;
            }

            self.example_js()
          }
        }
      },
    );
  }

  #[test]
  fn impl_with_map_value_result_catches() {
    test_macro(
      quote! {
        impl JsType {
          async fn example(&self) -> MapValue<
            Result<JsValue, JsValue>,
            Result<String, JsValue>,
          >;
        }
      },
      quote! {
        impl JsType {
          async fn example(&self) -> Result<String, JsValue> {
            #[::wasm_bindgen::prelude::wasm_bindgen]
            extern "C" {
              #[wasm_bindgen(method)]
              #[wasm_bindgen(js_name = "example")]
              #[wasm_bindgen(catch)]
              async fn example_js(this: &JsType) -> Result<JsValue, JsValue>;
            }

            self.example_js().await
          }
        }
      },
    );
  }

  #[test]
  fn impl_with_module_gets_applied() {
    test_macro(
      quote! {
        #[opts(module = "my-module")]
        impl JsType {
          fn example();
        }
      },
      quote! {
        impl JsType {
          fn example() {
            #[::wasm_bindgen::prelude::wasm_bindgen(module = "my-module")]
            extern "C" {
              #[wasm_bindgen(static_method_of = JsType)]
              #[wasm_bindgen(js_name = "example")]
              fn example_js();
            }

            Self::example_js()
          }
        }
      },
    );
  }
}
