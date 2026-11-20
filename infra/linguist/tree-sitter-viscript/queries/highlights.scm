; SPDX-License-Identifier: MIT OR Apache-2.0
; Khung highlight — chỉnh sau khi có node-types.json từ `tree-sitter generate`.

(comment) @comment

(string) @string
(number) @number

[
  "let"
  "var"
  "fn"
  "type"
  "enum"
  "trait"
  "impl"
  "use"
  "if"
  "elif"
  "else"
  "for"
  "in"
  "while"
  "loop"
  "match"
  "return"
  "break"
  "continue"
  "test"
  "and"
  "or"
  "not"
] @keyword

[
  "Some"
  "None"
  "Ok"
  "Err"
  "true"
  "false"
] @constant.builtin

(function name: (identifier) @function)
(call_expression function: (identifier) @function)

(let_statement name: (identifier) @variable)
(var_statement name: (identifier) @variable)
(parameter name: (identifier) @variable.parameter)
(field_declaration name: (identifier) @property)
(member_expression property: (identifier) @property)
