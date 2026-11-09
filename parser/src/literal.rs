use super::ast::{Expr, Literal};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, tag, take_while},
    character::complete::{char, digit1, line_ending, none_of, satisfy},
    character::{is_alphabetic, is_alphanumeric},
    combinator::{fail, map, map_res, not, opt, peek, recognize, value},
    error::{FromExternalError, ParseError},
    sequence::{delimited, terminated, tuple},
    IResult,
};

pub(crate) fn ident_char<'i, E: ParseError<&'i str>>(
    input: &'i str,
) -> IResult<&'i str, char, E> {
    satisfy(|c| c.is_ascii_alphanumeric() || c == '_')(input)
}

pub(crate) fn parse_bool<'i, E: ParseError<&'i str>>(
    input: &'i str,
) -> IResult<&'i str, Literal, E> {
    alt((
        value(Literal::BoolLiteral(true), terminated(tag("true"), peek(not(ident_char)))),
        value(Literal::BoolLiteral(false), terminated(tag("false"), peek(not(ident_char)))),
    ))(input)
}

pub(crate) fn parse_number<'i, E>(input: &'i str) -> IResult<&'i str, Literal, E>
where
    E: ParseError<&'i str> + FromExternalError<&'i str, std::num::ParseFloatError>,
{
    map_res(
        recognize(tuple((
            opt(char('-')),
            digit1,
            opt(tuple((char('.'), digit1))),
            opt(tuple((
                alt((char('E'), char('e'))),
                opt(alt((char('+'), char('-')))),
                digit1,
            ))),
        ))),
        |float_str: &'i str| float_str.parse::<f32>().map(Literal::NumLiteral),
    )(input)
}

pub(crate) fn parse_string<'i, E>(input: &'i str) -> IResult<&'i str, Expr, E>
where
    E: ParseError<&'i str>,
{
    use std::borrow::Cow;
    map(
        delimited(
            alt((char('"'), char('\''))),
            escaped_transform(
                none_of("\"\\"),
                '\\',
                alt((
                    value("\\", char('\\')),
                    value("", line_ending),
                    value("\t", char('t')),
                    value("\"", char('"')),
                    value("\n", char('n')),
                )),
            ),
            alt((char('"'), char('\''))),
        ),
        |parsed_str: String| {
            Expr::ExprLiteral(Literal::StringLiteral(Cow::Owned(parsed_str.to_string())))
        },
    )(input)
}

pub(crate) fn parse_identifier<'i, E>(input: &'i str) -> IResult<&'i str, &'i str, E>
where
    E: ParseError<&'i str>,
{
    if let Some(chr) = input.chars().next() {
        if !is_alphabetic(chr as u8) {
            return fail(input);
        }
    } else {
        return fail(input);
    }
    take_while(|c| is_alphanumeric(c as u8) || (c as char == '_'))(input)
}
