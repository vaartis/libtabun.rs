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

use ::{TClient,Comment,TabunError,CommentType};
use select::predicate::{And,Class,Name};

use std::collections::HashMap;
use regex::Regex;

impl<'a> TClient<'a> {

    ///Получить комменты из некоторого поста
    ///в виде HashMap ID-Коммент. Если блог указан как ""
    ///и пост указан как 0, то получает из `/comments/`
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_comments("/blog/lighthouse/157807.html");
    ///```
    pub fn get_comments(&mut self,url: &str) -> Result<HashMap<u32,Comment>,TabunError> {
        let mut ret = HashMap::new();
        let mut url = url.to_string();

        let url = &(if url.is_empty() {
            "/comments".to_owned()
        } else {
            if !url.starts_with('/') {
                let old_url = url.clone();
                url = "/".to_owned();
                url.push_str(&old_url);
            }
            url
        });

        let page = try!(self.get(url));

        let comments = page.find(And(Name("div"),Class("comments")));

        let url_regex = Regex::new(r"(\d+).html$").unwrap();

        for comm in comments.find(Class("comment")).iter() {
            let post_id = if url == "/comments" {
                let c = comm.find(Class("comment-path-topic"))
                    .first()
                    .unwrap();
                url_regex.captures(c.attr("href").unwrap())
                    .unwrap()
                    .at(1)
                    .unwrap()
                    .parse::<u32>()
                    .unwrap()
            } else {
                url_regex.captures(url)
                    .unwrap()
                    .at(1)
                    .unwrap()
                    .parse::<u32>()
                    .unwrap()
            };

            let id = match comm.find(And(Name("li"),Class("vote"))).first() {
                Some(x) => x.attr("id").unwrap().split('_').collect::<Vec<_>>()[3].parse::<u32>().unwrap(),
                None => comm.attr("id").unwrap().split('_').collect::<Vec<_>>()[2].parse::<u32>().unwrap()
            };

            if comm.attr("class").unwrap().contains("comment-bad") || comm.attr("class").unwrap().contains("comment-deleted") {
                ret.insert(id,Comment{
                    body:       String::new(),
                    id:         id,
                    author:     String::new(),
                    date:       String::new(),
                    votes:      0,
                    parent:     0,
                    post_id:    post_id,
                    deleted:    true
                });
                continue
            }

            let parent = match comm.find(Class("goto-comment-parent")).first() {
                Some(x) => {
                    let c = x.find(Name("a")).first().unwrap();
                    let c = c.attr("href").unwrap().split('/').collect::<Vec<_>>();
                    c[c.len()-1].parse::<u32>().unwrap() },
                None => 0
            };

            let text = comm.find(And(Name("div"),Class("text"))).first().unwrap().inner_html();
            let text = text.as_str();

            let author = comm.find(And(Name("li"),Class("comment-author")))
                .find(Name("a"))
                .first()
                .unwrap();
            let author = author.attr("href").unwrap().split('/').collect::<Vec<_>>()[4];

            let date = comm.find(Name("time")).first().unwrap();
            let date = date.attr("datetime").unwrap();

            let votes = match comm.find(And(Name("span"),Class("vote-count"))).first() {
                Some(x) => x.text().parse::<i32>().unwrap(),
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
                deleted:    false
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
    pub fn comment(&mut self,post_id: u32, body : &str, reply: u32, typ: CommentType) -> Result<u32,TabunError>{
        use mdo::option::{bind};

        let id_regex = Regex::new("\"sCommentId\":(\\d+)").unwrap();
        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();

        let url = format!("/{typ}/ajaxaddcomment?security_ls_key={key}&cmt_target_id={post_id}&reply={reply}&comment_text={text}",
                          text      = body,
                          post_id   = post_id,
                          reply     = reply,
                          typ       = match typ { CommentType::Post => "blog", CommentType::Talk => "talk" },
                          key       = self.security_ls_key);

        let res = try!(self.get(&url));

        let res = res.nth(0).unwrap().text();
        let res = res.as_str();

        if err_regex.is_match(res) {
            let err = err_regex.captures(res).unwrap();
            return Err(TabunError::Error(err.at(1).unwrap().to_owned(),err.at(2).unwrap().to_owned()));
        }

        Ok(mdo!(
            captures    =<< id_regex.captures(res);
            r           =<< captures.at(1);
            ret r.parse::<u32>().ok()
        ).unwrap())
    }

    ///Подписаться/отписаться от комментариев к посту.
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comments_subscribe(157198,false);
    ///```
    pub fn comments_subscribe(&mut self, post_id: u32, subscribed: bool) {
        let subscribed = if subscribed { "1" } else { "0" };

        let post_id = post_id.to_string();
        let key = self.security_ls_key.clone();

        let body = map![
        "target_type"       =>  "topic_new_comment",
        "target_id"         =>  post_id.as_str(),
        "value"             =>  subscribed,
        "mail"              =>  "",
        "security_ls_key"   => &key
        ];

        let _ = self.multipart("/subscribe/ajax-subscribe-toggle",body);
    }
}
