/* Posts
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
extern crate select;
extern crate hyper;

use ::{TClient,TabunError,Post,EditablePost,HOST_URL};

use select::predicate::{Name,Class,Attr,And};
use hyper::header::Referer;

use regex::Regex;
use std::str;

impl<'a> TClient<'a> {

    ///Создаёт пост в указанном блоге и возвращает его номер
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("computers").unwrap();
    ///user.add_post(blog_id,"Название поста","Текст поста",&vec!["тэг раз","тэг два"]);
    ///```
    pub fn add_post(&mut self, blog_id: u32, title: &str, body: &str, tags: &[&str]) -> Result<u32,TabunError> {
        use mdo::option::bind;

        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.clone();
        let tags = tags.iter().fold(String::new(), |acc, x| format!("{},{}", acc, x));

        let bd = map![
            "topic_type"            =>  "topic",
            "blog_id"               =>  &blog_id,
            "topic_title"           =>  title,
            "topic_text"            =>  body,
            "topic_tags"            =>  &tags,
            "submit_topic_publish"  =>  "Опубликовать",
            "security_ls_key"       =>  &key
        ];

        let res = try!(self.multipart("/topic/add",bd));

        let r = str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        Ok(mdo!(
                regex       =<< Regex::new(r"(\d+).html$").ok();
                captures    =<< regex.captures(r);
                r           =<< captures.at(1);
                ret r.parse::<u32>().ok()
               ).unwrap())
    }

    ///Получает посты из блога
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_posts("lighthouse",1);
    ///```
    pub fn get_posts(&mut self, blog_name: &str, page: u32) -> Result<Vec<Post>,TabunError>{
        let res = try!(self.get(&format!("/blog/{}/page{}", blog_name, page)));
        let mut ret = Vec::new();

        for p in res.find(Name("article")).iter() {
            let post_id = p.find(And(Name("div"),Class("vote-topic")))
                .first()
                .unwrap()
                .attr("id")
                .unwrap()
                .split('_').collect::<Vec<_>>()[3].parse::<u32>().unwrap();

            let post_title = p.find(And(Name("h1"),Class("topic-title")))
                .first()
                .unwrap()
                .text();

            let post_body = p.find(And(Name("div"),Class("topic-content")))
                .first()
                .unwrap()
                .inner_html();
            let post_body = post_body.trim();

            let post_date = p.find(And(Name("li"),Class("topic-info-date")))
                .find(Name("time"))
                .first()
                .unwrap();
            let post_date = post_date.attr("datetime")
                .unwrap();

            let mut post_tags = Vec::new();
            for t in res.find(And(Name("a"),Attr("rel","tag"))).iter() {
                post_tags.push(t.text());
            }

            let cm_count = p.find(And(Name("li"),Class("topic-info-comments")))
                .first()
                .unwrap()
                .find(Name("span")).first().unwrap().text()
                .parse::<u32>().unwrap();

            let post_author = res.find(And(Name("div"),Class("topic-info")))
                .find(And(Name("a"),Attr("rel","author")))
                .first()
                .unwrap()
                .text();
            ret.push(
                Post{
                    title:          post_title,
                    body:           post_body.to_owned(),
                    date:           post_date.to_owned(),
                    tags:           post_tags,
                    comments_count: cm_count,
                    author:         post_author,
                    id:             post_id, });
        }
        Ok(ret)
    }

    ///Получает EditablePost со страницы редактирования поста
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_editable_post(1111);
    ///```
    pub fn get_editable_post(&mut self, post_id: u32) -> Result<EditablePost,TabunError> {
        let res = try!(self.get(&format!("/topic/edit/{}",post_id)));

        let title = res.find(Attr("id","topic_title")).first().unwrap();
        let title = title.attr("value").unwrap().to_string();

        let tags = res.find(Attr("id","topic_tags")).first().unwrap();
        let tags = tags.attr("value").unwrap();
        let tags = tags.split(',').map(|x| x.to_string()).collect::<Vec<String>>();

        Ok(EditablePost{
            title:  title,
            body:   res.find(Attr("id","topic_text")).first().unwrap().text(),
            tags:   tags.clone()
        })
    }

    ///Получает пост, блог можно опустить (передать `""`), но лучше так не делать,
    ///дабы избежать доволнительных перенаправлений.
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_post("computers",157198);
    /// //или
    ///user.get_post("",157198);
    ///```
    pub fn get_post(&mut self,blog_name: &str,post_id: u32) -> Result<Post,TabunError>{
        let res = if blog_name.is_empty() {
            try!(self.get(&format!("/blog/{}.html",post_id)))
        } else {
            try!(self.get(&format!("/blog/{}/{}.html",blog_name,post_id)))
        };

        let post_title = res.find(And(Name("h1"),Class("topic-title")))
            .first()
            .unwrap()
            .text();

        let post_body = res.find(And(Name("div"),Class("topic-content")))
            .first()
            .unwrap()
            .inner_html();
        let post_body = post_body.trim();

        let post_date = res.find(And(Name("li"),Class("topic-info-date")))
            .find(Name("time"))
            .first()
            .unwrap();
        let post_date = post_date.attr("datetime")
            .unwrap();

        let mut post_tags = Vec::new();
        for t in res.find(And(Name("a"),Attr("rel","tag"))).iter() {
            post_tags.push(t.text());
        }

        let cm_count = res.find(And(Name("span"),Attr("id","count-comments")))
            .first()
            .unwrap()
            .text()
            .parse::<u32>()
            .unwrap();

        let post_author = res.find(And(Name("div"),Class("topic-info")))
            .find(And(Name("a"),Attr("rel","author")))
            .first()
            .unwrap()
            .text();

        Ok(Post{
            title:          post_title,
            body:           post_body.to_owned(),
            date:           post_date.to_owned(),
            tags:           post_tags,
            comments_count: cm_count,
            author:         post_author,
            id:             post_id,
        })
    }

    ///Редактирует пост, возвращает его ID
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("computers").unwrap();
    ///user.edit_post(157198,blog_id,"Новое название", "Новый текст", &vec!["тэг".to_string()],false);
    ///```
    pub fn edit_post(&mut self, post_id: u32, blog_id: u32, title: &str, body: &str, tags: &[String], forbid_comment: bool) -> Result<u32,TabunError> {
        use mdo::option::{bind};

        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.clone();
        let forbid_comment = if forbid_comment { "1" } else { "0" };
        let tags = tags.iter().fold(String::new(), |acc, x| format!("{},{}", acc, x));

        let bd = map![
            "topic_type"            =>  "topic",
            "blog_id"               =>  &blog_id,
            "topic_title"           =>  title,
            "topic_text"            =>  body,
            "topic_tags"            =>  &tags,
            "submit_topic_publish"  =>  "Опубликовать",
            "security_ls_key"       =>  &key,
            "topic_forbid_comment"  =>  &forbid_comment
        ];

        let res = try!(self.multipart(&format!("/topic/edit/{}",post_id), bd));

        let r = str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        Ok(mdo!(
            regex       =<< Regex::new(r"(\d+).html$").ok();
            captures    =<< regex.captures(r);
            r           =<< captures.at(1);
            ret r.parse::<u32>().ok()
        ).unwrap())
    }

    ///Удаляет пост, и, так как табун ничего не возаращет по этому поводу,
    ///выдаёт Ok(true) в случае удачи
    pub fn delete_post(&mut self, post_id: u32) -> Result<bool,TabunError> {
        let url = format!("/topic/delete/{}/?security_ls_key={}", post_id ,&self.security_ls_key);
        match self.create_middle_req(&url)
            .header(Referer(format!("{}/blog/{}.html", HOST_URL, post_id)))
            .send().unwrap().status {
                hyper::Ok => Ok(true),
                x => Err(TabunError::NumError(x))
            }
    }
}
