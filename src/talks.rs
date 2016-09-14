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

use ::{TClient,TabunError,Talk,TalkItem,HOST_URL};

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

impl<'a> TClient<'a> {

    ///Получает личный диалог по его ID
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_talk(123);
    pub fn get_talk(&mut self, talk_id: u32) -> Result<Talk,TabunError>{
        let url = format!("/talk/read/{}", talk_id);
        let page = try!(self.get(&url));

        let title = page.find(Class("topic-title"))
            .first()
            .unwrap()
            .text();

        let body = page.find(Class("topic-content"))
            .first()
            .unwrap()
            .inner_html();
        let body = body.trim().to_string();

        let date = page.find(And(Name("li"),Class("topic-info-date")))
            .find(Name("time"))
            .first()
            .unwrap();
        let date = date.attr("datetime")
            .unwrap()
            .to_string();

        let comments = try!(self.get_comments(&url));

        let users = page.find(Class("talk-recipients-header"))
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
        let key = self.security_ls_key.clone();

        let fields = map![
            "submit_talk_add" => "Отправить",
            "security_ls_key" => &key,
            "talk_users" => &users,
            "talk_title" => &title,
            "talk_text" => &body
        ];

        let res = try!(self.multipart("/talk/add",fields));

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
    pub fn get_talks(&mut self, page: u32) -> Result<Vec<TalkItem>, TabunError> {
        let res = try!(self.get(&format!("/talk/inbox/page{}", page)));
        let mut ret = Vec::new();

        let res = res.find(Name("tbody"));

        for p in res.find(Name("tr")).iter() {
            let talk_id = p.find(And(Name("a"), Class("js-title-talk")))
                .first()
                .unwrap()
                .attr("href")
                .unwrap()
                .split('/').collect::<Vec<_>>()[5].parse::<u32>().unwrap();

            let talk_title = p.find(And(Name("a"), Class("js-title-talk")))
                .first()
                .unwrap()
                .text();

            let talk_users = p.find(And(Name("td"), Class("cell-recipients")))
                .find(And(Name("a"), Class("username")))
                .iter()
                .map(|x| x.text().to_string())
                .collect::<Vec<_>>();

                ret.push(
                    TalkItem {
                        id: talk_id,
                        title: talk_title,
                        users: talk_users,
                    });
        }
        Ok(ret)
    }

    ///Удаляет цепочку сообщений, и, так как табун ничего не возаращет по этому поводу,
    ///выдаёт Ok(true) в случае удачи
    pub fn delete_talk(&mut self, talk_id: u32) -> Result<bool,TabunError> {
        let url = format!("/talk/delete/{}/?security_ls_key={}", talk_id ,&self.security_ls_key);
        match self.create_middle_req(&url)
            .header(Referer(format!("{}/talk/{}/", HOST_URL, talk_id)))
            .send().unwrap().status {
                hyper::Ok => Ok(true),
                x => Err(TabunError::NumError(x))
            }
    }
}
