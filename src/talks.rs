/* Talks
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

extern crate regex;
extern crate hyper;

use super::*;

use select::document::Document;
use select::predicate::{Class, Name, And};

use std::{str,convert};

use regex::Regex;

use hyper::header::Referer;

pub enum TalkError {
    NoMembers,
    TabunError(TabunError)
}

impl convert::From<hyper::status::StatusCode> for TalkError {
    fn from(x: hyper::status::StatusCode) -> TalkError {
        TalkError::TabunError(TabunError::NumError(x))
    }
}

impl convert::From<TabunError> for TalkError {
    fn from(x: TabunError) -> TalkError {
        TalkError::TabunError(x)
    }
}

impl<'a> TClient<'a> {

    ///Получает личный диалог по его ID
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_talk(123);
    pub fn get_talk(&mut self, talk_id: u32) -> TabunResult<Talk>{
        let url = format!("/talk/read/{}/", talk_id);
        let doc = try!(self.get_document(&url));
        self.doc_get_talk(&doc, talk_id)
    }

    pub fn doc_get_talk<T: Into<Option<u32>>>(&mut self, doc: &Document, talk_id: T) -> TabunResult<Talk>{
        let title = try_to_parse!(doc.find(Class("topic-title")).first()).text();

        let body = try_to_parse!(doc.find(Class("topic-content")).first()).inner_html();
        let body = body.trim().to_string();

        let date = try_to_parse!(doc.find(And(Name("li"),Class("topic-info-date")))
                                 .find(Name("time"))
                                 .first());
        let date = try_to_parse!(date.attr("datetime")).to_string();

        let comments = try!(match talk_id.into() {
            Some(t) => self.doc_get_comments(&doc, format!("/talk/read/{}/", t).as_str()),
            None => self.doc_get_comments(&doc, None)
        });

        let users = doc.find(Class("talk-recipients-header"))
            .find(Name("a"))
            .iter()
            .filter(|x| x.attr("class").unwrap().contains("username"))
            .map(|x| x.text().to_string())
            .collect::<Vec<_>>();

        Ok(Talk{
            title:      title,
            body:       body,
            comments:   comments,
            users:      users,
            date:       date
        })
    }

    ///Создаёт личный диалог с пользователями и возвращает ID диалога
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.add_talk(&vec!["человек1","человек2"], "Название", "Текст");
    pub fn add_talk(&mut self, users: &[&str], title: &str, body:&str ) -> Result<u32,TalkError> {

        let users = users.iter().fold(String::new(),|acc, x| format!("{},{}",acc, x));
        let key = self.security_ls_key.to_owned();

        let fields = map![
            "submit_talk_add" => "Отправить",
            "security_ls_key" => &key,
            "talk_users" => &users,
            "talk_title" => &title,
            "talk_text" => &body
        ];

        let res = try!(self.post_multipart("/talk/add",fields));

        if let Some(x) = res.headers.get_raw("location") {
            parse_text_to_res!(regex => r"read/(\d+)/$", st => str::from_utf8(&x[0]).unwrap(), num => 1, typ => u32)
        } else {
            Err(TalkError::NoMembers)
        }
    }

    ///Получить список личных сообщений
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_talks(1);
    ///```
    pub fn get_talks(&mut self, page: u32) -> TabunResult<Vec<TalkItem>> {
        let doc = try!(self.get_document(&format!("/talk/inbox/page{}/", page)));
        self.doc_get_talks(&doc)
    }

    pub fn doc_get_talks(&mut self, doc: &Document) -> TabunResult<Vec<TalkItem>> {
        let mut ret = Vec::new();

        let res = doc.find(Name("tbody"));

        for p in res.find(Name("tr")).iter() {
            let talk_id = try_to_parse!(hado!{
                el <- p.find(And(Name("a"), Class("js-title-talk"))).first();
                attr <- el.attr("href");
                attr.split('/').collect::<Vec<_>>()[5].parse::<u32>().ok()
            });

            let talk_title = try_to_parse!(p.find(And(Name("a"), Class("js-title-talk"))).first()).text();

            let talk_users = p.find(And(Name("td"), Class("cell-recipients")))
                .find(And(Name("a"), Class("username")))
                .iter()
                .map(|x| x.text().to_string())
                .collect::<Vec<_>>();

                ret.push(TalkItem {
                    id: talk_id,
                    title: talk_title,
                    users: talk_users,
                });
        }
        Ok(ret)
    }

    ///Удаляет цепочку сообщений, и, так как табун ничего не возаращет по этому поводу,
    ///выдаёт `Ok(())` в случае удачи
    pub fn delete_talk(&mut self, talk_id: u32) -> TabunResult<()> {
        let url = format!("/talk/delete/{}/?security_ls_key={}", talk_id ,&self.security_ls_key);
        match self.create_middle_req(&url)
            .header(Referer(format!("{}/talk/read/{}/", HOST_URL, talk_id)))
            .send().unwrap().status {
                hyper::Ok => Ok(()),
                x => Err(TabunError::NumError(x))
            }
    }
}
