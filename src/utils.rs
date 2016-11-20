/* Utilities, macro and other helpers
 *
 * Copyright (C) 2016 TyanNN <TyanNN@cocaine.ninja>
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with this program; if not, write to the Free Software Foundation, Inc.,
 * 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
*/

macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

///Макро для парса строк и возврата Result,
///парсит st указанным regex, затем вынимает группу номер num
///и парсит в typ
macro_rules! parse_text_to_res(
    { $(regex => $regex:expr, st => $st:expr, num => $n:expr, typ => $typ:ty)+ } => {
        {
            $(
                match hado! {
                    reg <- Regex::new($regex).ok();
                    captures <- reg.captures($st);
                    at <- captures.at($n);
                    at.parse::<$typ>().ok() } {
                        Some(x) => Ok(x),
                        None    => unreachable!()
                    }
            )+
        }
    };
);

///Макро для удобного unescape
macro_rules! unescape(
    { $($x:expr)+ } => {
        {
            $(
                match unescape::unescape($x) {
                    Some(x) => x,
                    None    => unreachable!()
                }
             )+
        }
    };
);

///Макрос, создающий TabunError::ParseError
macro_rules! parse_error {
    () => {
        TabunError::ParseError(
            String::from(file!()), line!(), String::from("Cannot parse response from server")
        )
    };
    ( $msg: expr ) => {
        TabunError::ParseError(
            String::from(file!()), line!(), String::from($msg)
        )
    };
}

///Макрос для возвращения ошибок парсинга
macro_rules! try_to_parse {
    ( $expr: expr ) => {
        match $expr {
            Some(x) => x,
            None => return Err(parse_error!()),
        }
    };
    ( $expr: expr, $msg: expr ) => {
        match $expr {
            Some(x) => x,
            None => return Err(parse_error!($msg)),
        }
    };
}

///Макрос для парсинга json-объекта
macro_rules! try_to_parse_json {
    ( $expr: expr ) => {
        {
            let tmp: Value = try_to_parse!(
                serde_json::from_str($expr).ok(),
                "Cannot parse JSON object"
            );
            if tmp.is_object() {
                tmp
            } else {
                return Err(parse_error!("Expected JSON object"))
            }
        }
    }
}

///Макрос для получения из json-объекта значения определенного типа
///или значения по умолчанию
macro_rules! get_json {
    ($expr: expr, $key: expr, $as_who: ident, $def: expr) => {
        match $expr.pointer($key) {
            Some(x) => x.$as_who().unwrap_or($def),
            None => $def,
        }
    };

    ( $expr: expr, $key: expr, $as_who: ident ) => {
        match $expr.pointer($key) {
            Some(x) => x.$as_who(),
            None => None,
        }
    }
}

/// Возвращает подстроку, находящуюся между кусками `start` и `end`.
///
/// Параметры `with_start` и `with_end` указывают, включать ли сами куски
/// `start` и `end` соответственно в результат.
///
/// При `extend` = `true` кусок `end` будет искаться с конца исходной строки,
/// а не с начала.
///
///
/// # Examples
///
/// ```
/// let result = libtabun::utils::find_substring(
///     "<a><b>c</b><b>d</b></a>",
///     "<b>", false,
///     "</b>", false,
///     false
/// );
/// assert_eq!(result, Some("c".to_string()));
/// ```
///
/// ```
/// let result = libtabun::utils::find_substring(
///     "<a><b>c</b><b>d</b></a>",
///     "<b>", true,
///     "</b>", true,
///     true
/// );
/// assert_eq!(result, Some("<b>c</b><b>d</b>".to_string()));
/// ```
///
/// ```
/// let result = libtabun::utils::find_substring(
///     "<a><b>c</b><b>d</b></a>",
///     "<b>", true,
///     "not-exist", true,
///     true
/// );
/// assert_eq!(result, None);
/// ```
pub fn find_substring(s: &str, start: &str, with_start: bool, end: &str, with_end: bool, extend: bool) -> Option<String> {
    let f1 = match s.find(start) {
        None => return None,
        Some(x) => x
    };
    let (_, s) = s.split_at(if with_start { f1 } else { f1 + start.len() });

    let f2: usize;
    if extend {
        f2 = match s.rfind(end) {
            None => return None,
            Some(x) => x
        };
    } else {
        f2 = match s.find(end) {
            None => return None,
            Some(x) => x
        };
    }

    let (result, _) = s.split_at(if with_end { f2 + end.len() } else { f2 });
    Some(result.to_string())
}
