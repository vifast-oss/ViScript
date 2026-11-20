// SPDX-License-Identifier: MIT OR Apache-2.0
// KHUNG — chưa sinh parser. Chạy `npx tree-sitter generate` trước khi dùng.
// Phạm vi: đủ để tô màu (keyword/string/số/comment) cho README và tệp .vis.

module.exports = grammar({
  name: 'viscript',

  extras: $ => [/\s/, $.comment],

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice(
      $.use_statement,
      $.fn_definition,
      $.test_definition,
      $.type_definition,
      $.enum_definition,
      $.trait_definition,
      $.impl_definition,
      $.let_statement,
      $.var_statement,
      $.if_statement,
      $.for_statement,
      $.while_statement,
      $.loop_statement,
      $.match_statement,
      $.return_statement,
      $.break_statement,
      $.continue_statement,
      $.expression_statement,
    ),

    comment: $ => token(seq('#', /[^\n]*/)),

    use_statement: $ => seq('use', field('path', $.identifier)),

    fn_definition: $ => seq(
      'fn', field('name', $.identifier),
      optional($.parameter_list), optional($.return_type), ':',
      field('body', $.block),
    ),

    test_definition: $ => seq(
      'test', 'fn', field('name', $.identifier), '(', ')', ':',
      field('body', $.block),
    ),

    type_definition: $ => seq('type', field('name', $.identifier), choice(
      seq('=', field('target', $._type)),
      seq('(', field('inner', $._type), ')'),
      $.field_block,
    )),

    enum_definition: $ => seq('enum', field('name', $.identifier), ':', $.variant_block),
    trait_definition: $ => seq('trait', field('name', $.identifier), ':', repeat($.method_signature)),
    impl_definition: $ => seq('impl', field('trait', $.identifier), 'for', field('type', $.identifier), ':', repeat($.fn_definition)),

    field_block: $ => seq(repeat1($.field_declaration)),
    variant_block: $ => seq(repeat1($.variant_declaration)),
    field_declaration: $ => seq(field('name', $.identifier), ':', $._type, optional(seq('=', $._expression))),
    variant_declaration: $ => seq(field('name', $.identifier), optional($.parameter_list)),
    method_signature: $ => seq('fn', field('name', $.identifier), optional($.parameter_list), optional($.return_type)),

    // Thụt lề là phần của ngữ nghĩa ViScript. Để parse chính xác cần external
    // scanner (scanner.c) sinh INDENT/DEDENT; ở khung tô màu này chỉ cần nhóm câu.
    block: $ => repeat($._statement),

    parameter_list: $ => seq('(', commaSep($.parameter), ')'),
    parameter: $ => seq(field('name', $.identifier), optional(seq(':', $._type))),
    return_type: $ => seq('->', $._type),

    let_statement: $ => seq('let', field('name', $.identifier), optional(seq(':', $._type)), '=', $._expression),
    var_statement: $ => seq('var', field('name', $.identifier), optional(seq(':', $._type)), '=', $._expression),

    if_statement: $ => seq(
      'if', $._expression, ':', field('body', $.block),
      repeat(seq('elif', $._expression, ':', $.block)),
      optional(seq('else', ':', $.block)),
    ),

    for_statement: $ => seq('for', field('name', $.identifier), 'in', $._expression, ':', field('body', $.block)),
    while_statement: $ => seq('while', $._expression, ':', field('body', $.block)),
    loop_statement: $ => seq('loop', ':', field('body', $.block)),

    match_statement: $ => seq('match', $._expression, ':', repeat1($.match_arm)),
    match_arm: $ => seq($.pattern, ':', choice($.block, $._expression)),
    pattern: $ => choice($.identifier, $.constructor_pattern, $.wildcard_pattern),
    constructor_pattern: $ => seq($.identifier, '(', commaSep($.pattern), ')'),
    wildcard_pattern: $ => '_',

    return_statement: $ => seq('return', optional($._expression)),
    break_statement: $ => 'break',
    continue_statement: $ => 'continue',
    expression_statement: $ => $._expression,

    _expression: $ => choice(
      $.identifier, $.number, $.string, $.list, $.map, $.call_expression,
      $.member_expression, $.index_expression, $.binary_expression,
      $.unary_expression, $.closure, $.constructor_expression, $.parenthesized_expression,
    ),

    identifier: $ => /[\p{L}_][\p{L}\p{N}_]*/u,
    number: $ => token(/[0-9]+(\.[0-9]+)?(kb|kib|mb|mib|gb|gib|h|m|s|ms)?/),
    string: $ => token(seq('"', repeat(choice(/[^"\\{}]/, /\\./, seq('{', /[^}]*/, '}'))), '"')),
    list: $ => seq('[', commaSep($._expression), ']'),
    map: $ => seq('{', commaSep(seq($._expression, ':', $._expression)), '}'),

    call_expression: $ => seq(field('function', $._expression), field('arguments', $.argument_list)),
    argument_list: $ => seq('(', commaSep(seq(optional(seq($.identifier, ':')), $._expression)), ')'),
    member_expression: $ => seq(field('object', $._expression), '.', field('property', $.identifier)),
    index_expression: $ => seq(field('object', $._expression), '[', field('index', $._expression), optional('?'), ']'),

    binary_expression: $ => choice(
      ...[
        ['??', 1], ['or', 2], ['and', 3],
        ['==', 4], ['!=', 4], ['<', 4], ['<=', 4], ['>', 4], ['>=', 4],
        ['+', 5], ['-', 5], ['*', 6], ['/', 6], ['//', 6], ['%', 6],
      ].map(([op, prec]) => prec.left(prec, seq(field('left', $._expression), op, field('right', $._expression)))),
    ),
    unary_expression: $ => prec(7, seq(choice('-', 'not'), field('argument', $._expression))),
    closure: $ => seq(field('parameter', $.identifier), '=>', field('body', $._expression)),
    constructor_expression: $ => seq(field('name', $.identifier), '(', commaSep($._expression), ')'),
    parenthesized_expression: $ => seq('(', $._expression, ')'),

    _type: $ => choice(
      $.identifier,
      seq($.identifier, '[', $._type, ']'),
      seq($._type, '?'),
      seq($.identifier, '.', $.identifier),
    ),

  },
});

function commaSep(rule) {
  return optional(seq(rule, repeat(seq(',', rule)), optional(',')));
}
