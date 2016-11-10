/* Comments
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

extern crate select;
extern crate regex;
extern crate unescape;

use super::*;
use select::document::Document;
use select::predicate::{And,Class,Name};

use std::collections::HashMap;
use regex::Regex;

impl<'a> TClient<'a> {

    ///Получить комменты из некоторого поста/сообщения
    ///в виде HashMap ID-Коммент. Если ссылка указана как None,
    ///то получает из `/comments/`
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_comments("/blog/lighthouse/157807.html");
    ///```
    pub fn get_comments<'f, T: Into<Option<&'f str>>>(&mut self, url: T) -> TabunResult<HashMap<u32, Comment>> {
        let url = &(match url.into() {
            None    => "/comments".to_owned(),
            Some(x) => {
                if !x.starts_with('/') {
                    format!("/{}",x)
                } else {
                    x.to_owned()
                }
            }
        });

        let doc = try!(self.get_document(url));
        self.doc_get_comments(&doc, url.as_str())
    }

    ///Получить комменты из некоторого поста/сообщения
    ///в виде HashMap ID-Коммент. Если ссылка указана как None,
    ///то получает из `/comments/`. Строку url требуется передать для
    ///комментов не из ленты, потому что в них не содержится информации о
    ///самом посте; если сохранить post_id не слишком важно, url можно
    ///не передавать.
    ///```
    pub fn doc_get_comments<'f, T: Into<Option<&'f str>>>(&mut self, doc: &Document, url: T) -> TabunResult<HashMap<u32, Comment>> {
        let mut ret = HashMap::new();

        let url = match url.into() {
            Some(x) => Some(x.to_string()),
            None => None,
        };

        let comments = doc.find(And(Name("div"),Class("comments")));

        let post_url_regex = Regex::new(r"(\d+).html$").unwrap();

        for comm in comments.find(Class("comment")).iter() {
            let path_info = comm.find(Class("comment-path-topic")).first();
            let href = match path_info {
                Some(p) => Some(try_to_parse!(p.attr("href")).to_string()),
                None => url.to_owned(),
            };

            let post_id = match href {
                Some(x) => {
                    if let Some(capts) = post_url_regex.captures(&x) {
                        try_to_parse!(hado!{
                            at <- capts.at(1);
                            at.parse::<u32>().ok()
                        })
                    } else {
                        0
                    }
                },
                None => 0
            };

            let id = try_to_parse!(match comm.find(And(Name("li"),Class("vote"))).first() {
                Some(x) => hado!{
                    attr <- x.attr("id");
                    id_s <- attr.split('_').collect::<Vec<_>>().get(3);
                    id_s.parse::<u32>().ok()
                },
                None => hado!{
                    attr <- comm.attr("id");
                    id_s <- attr.split('_').collect::<Vec<_>>().get(2);
                    id_s.parse::<u32>().ok()
                }
            });

            let cl = try_to_parse!(comm.attr("class"));
            if cl.contains("comment-bad") || cl.contains("comment-deleted") {
                ret.insert(id,Comment{
                    body:       String::new(),
                    id:         id,
                    author:     String::new(),
                    date:       String::new(),
                    votes:      0,
                    parent:     0,
                    post_id:    post_id,
                    deleted:    true,
                });
                continue
            }

            let parent = match comm.find(Class("goto-comment-parent")).first() {
                Some(x) => {
                    try_to_parse!(hado!{
                        el <- x.find(Name("a")).first();
                        attr <- el.attr("href");
                        pr_s <- attr.split('/').collect::<Vec<_>>().last();
                        pr_s.parse::<u32>().ok()
                    })},
                None => 0
            };

            let text = try_to_parse!(comm.find(And(Name("div"),Class("text"))).first()).inner_html();

            let author = try_to_parse!(comm.find(And(Name("li"),Class("comment-author"))).find(Name("a")).first());
            let author = try_to_parse!(author.attr("href")).split('/').collect::<Vec<_>>()[4];

            let date = try_to_parse!(comm.find(Name("time")).first());
            let date = try_to_parse!(date.attr("datetime"));

            let votes = match comm.find(And(Name("span"),Class("vote-count"))).first() {
                Some(x) => try_to_parse!(x.text().parse::<i32>().ok()),
                None    => 0
            };

            ret.insert(id,Comment{
                body:       text.to_owned(),
                id:         id,
                author:     author.to_owned(),
                date:       date.to_owned(),
                votes:      votes,
                parent:     parent,
                post_id:    post_id,
                deleted:    false,
            });
        }
        Ok(ret)
    }

    ///Оставить коммент к какому-нибудь посту, reply=0 - ответ на сам пост,
    ///иначе на чей-то коммент, возвращает ID нового коммента
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comment(1234,"Привет!", 0, libtabun::CommentType::Post);
    ///```
    pub fn comment(&mut self,post_id: u32, body : &str, reply: u32, typ: CommentType) -> TabunResult<u32>{
        let s_post_id = post_id.to_string();
        let s_reply = reply.to_string();

        let data = try!(self.ajax(
            &format!(
                "/{}/ajaxaddcomment/",
                match typ { CommentType::Post => "blog", CommentType::Talk => "talk" }
            ),
            map![
                "comment_text" => body,
                "cmt_target_id" => s_post_id.as_str(),
                "reply" => s_reply.as_str()
            ]
        ));

        match get_json!(data, "/sCommentId", as_u64) {
            Some(comment_id) => Ok(comment_id as u32),
            None => Err(parse_error!("Server did not return sCommentId"))
        }
    }

    ///Подписаться/отписаться от комментариев к посту.
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comments_subscribe(157198,false);
    ///```
    pub fn comments_subscribe(&mut self, post_id: u32, subscribed: bool) -> TabunResult<()> {
        let subscribed = if subscribed { "1" } else { "0" };

        let s_post_id = post_id.to_string();

        let body = map![
            "target_type"       =>  "topic_new_comment",
            "target_id"         =>  s_post_id.as_str(),
            "value"             =>  subscribed,
            "mail"              =>  ""
        ];

        try!(self.ajax("/subscribe/ajax-subscribe-toggle", body));
        Ok(())
    }

    ///Добавить комментарий в изранное или удалить его оттуда (true/false)
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.favourite_comment(12345, true);
    ///```
    pub fn favourite_comment(&mut self, id: u32, typ: bool) -> TabunResult<u32> {
        self.favourite(id, typ, true)
    }
}

#[cfg(test)]
mod test {
    use ::TClient;

    #[test]
    fn test_get_comments() {
        let mut user = TClient::new(None,None).unwrap();
        match user.get_comments("/blog/news/67052.html") { //Старый пост Орхи
            Ok(x)   => {
                assert!(x[&3927613].body.contains("нежданчик"));
                assert_eq!(x[&3927613].votes, 0);
                assert_eq!(x[&3927613].parent, 0);
                assert_eq!(x[&3927613].post_id, 67052);
            },
            Err(x)  => panic!(x)
        }
    }

    #[test]
    fn test_doc_get_comments() {
        let mut user = TClient::new(None, None).unwrap();
        let doc = user.get_document("/comments/").unwrap();
        match user.doc_get_comments(&doc, None) {
            Ok(comms) => {
                assert_eq!(comms.len(), 50);
                for (id, ref comm) in &comms {
                    assert_eq!(comm.id, *id);
                    assert!(comm.post_id > 0);
                }
            },
            Err(x)=> panic!(x)
        }
    }
}
