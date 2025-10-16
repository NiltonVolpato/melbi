// ============================================================================
// REVIEWED & LOCKED - Test expectations are set in stone
// Date: 2024-10-14
// All test expectations in this file have been reviewed and approved.
// DO NOT change expectations without explicit discussion.
// If tests fail, fix the formatter, not the tests.
// ============================================================================

mod cases;

use indoc::indoc;

test_case!(
    simple_lambda,
    input: "(x)=>x + 1",
    formatted: Ok("(x) => x + 1"),
);

test_case!(
    lambda_no_params,
    input: indoc! {"
        (

        ) =>
          42"},
    formatted: Ok("() => 42"),
);

test_case!(
    lambda_multiple_params,
    input: indoc! {"
        (x,y,z)
        =>
        x+y+z
    "},
    formatted: Ok("(x, y, z) => x + y + z"),
);

test_case!(
    lambda_multiple_params_with_comments,
    input: indoc! {"
        (
            name,  // user's name
            age,  // user's age
            email,  // user's email
        ) => {
            name = name,
            age = age,
            email = email,
        }
    "},
    formatted: Ok(indoc!{"
        (
            name,   // user's name
            age,    // user's age
            email,  // user's email
        ) => {
            name = name,
            age = age,
            email = email,
        }
    "}),
);

test_case!(
    lambda_trailing_comma,
    input: "(x,y,) => x+y",
    formatted: Ok("(x, y) => x + y"),
);

test_case!(
    lambda_with_where,
    input: "(a,b,c) => result where{delta=b^2-4*a*c,result=[1,2]}",
    formatted: Ok("(a, b, c) => result where { delta = b ^ 2 - 4 * a * c, result = [1, 2] }"),
);

test_case!(
    lambda_with_multiline_where,
    input: indoc! {"
        (a, b, c) => result where {
            delta = b^2 - 4*a*c, r0 = (-b + delta^0.5) / (2*a), r1 = (-b - delta^0.5) / (2*a), result = [r0, r1]
        }"},
    formatted: Ok(indoc! {"
        (a, b, c) => result where {
            delta = b ^ 2 - 4 * a * c,
            r0 = (- b + delta ^ 0.5) / (2 * a),
            r1 = (- b - delta ^ 0.5) / (2 * a),
            result = [r0, r1],
        }"}),
);

test_case!(
    lambda_with_multiline_where_hanging,
    input: indoc! {"
        (a, b, c) => result where {delta = b^2 - 4*a*c,
                                   r0 = (-b + delta^0.5) / (2*a),
                                   r1 = (-b - delta^0.5) / (2*a),
                                   result = [r0, r1]}"},
    formatted: Ok(indoc! {"
        (a, b, c) => result where {
            delta = b ^ 2 - 4 * a * c,
            r0 = (- b + delta ^ 0.5) / (2 * a),
            r1 = (- b - delta ^ 0.5) / (2 * a),
            result = [r0, r1],
        }"}),
);

test_case!(
    nested_lambda,
    input: "(x)=>(y)=>x+y",
    formatted: Ok("(x) => (y) => x + y"),
);
