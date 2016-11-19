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

use super::*;

use select::document::Document;
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
    ///let post_id = user.add_post(blog_id,"Название поста","Текст поста",&vec!["тэг раз","тэг два"]).unwrap();
    ///```
    pub fn add_post(&mut self, blog_id: u32, title: &str, body: &str, tags: &[&str]) -> TabunResult<u32> {
        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.to_owned();
        let tags = tags.iter().fold(String::new(), |acc, x| format!("{},{}", acc, x));

        let bd = vec![
            ("topic_type",            "topic"),
            ("blog_id",               &blog_id),
            ("topic_title",           title),
            ("topic_text",            body),
            ("topic_tags",            &tags),
            ("submit_topic_publish",  "Опубликовать"),
            ("security_ls_key",       &key)
        ];

        let res = try!(self.post_multipart("/topic/add", &bd));

        let r = str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();
        parse_text_to_res!(regex => r"(\d+).html$", st => r, num => 1, typ => u32 )
    }

    ///Создаёт опрос в указанном блоге и возвращает его номер
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("computers").unwrap();
    ///let post_id = user.add_poll(
    ///    blog_id, "Вопрос",
    ///    &vec!["Ответ 1", "Ответ 2", "Ответ 3"],
    ///    "Текст поста", &vec!["тэг раз", "тэг два"],
    ///    false, true
    ///).unwrap();
    ///```
    pub fn add_poll(&mut self, blog_id: u32, title: &str, choices: &[&str], body: &str, tags: &[&str], forbid_comment: bool, publish: bool) -> TabunResult<u32> {
        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.to_owned();
        let tags = tags.iter().fold(String::new(), |acc, x| format!("{},{}", acc, x));
        let forbid_comment = if forbid_comment { "1" } else { "0" };

        let mut bd = vec![
            ("topic_type",            "question"),
            ("blog_id",               &blog_id),
            ("topic_title",           title),
            ("topic_text",            body),
            ("topic_tags",            &tags),
            ("topic_forbid_comment",  forbid_comment),
            ("security_ls_key",       &key)
        ];

        for choice in choices {
            bd.push(("answer[]", choice));
        }

        if publish {
            bd.push(("submit_topic_publish", "Опубликовать"));
        } else {
            bd.push(("submit_topic_save", "Сохранить в черновиках"));
        }

        let res = try!(self.post_multipart("/question/add", &bd));

        let r = str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();
        parse_text_to_res!(regex => r"(\d+).html$", st => r, num => 1, typ => u32 )
    }

    ///Получает посты из блога
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_posts("lighthouse",1);
    ///```
    pub fn get_posts(&mut self, blog_name: &str, page: u32) -> TabunResult<Vec<Post>> {
        let doc = try!(self.get_document(&format!("/blog/{}/page{}/", blog_name, page)));
        self.doc_get_posts(&doc)
    }

    ///Получает посты со страницы
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let page = user.get_document("/profile/Orhideous/created/posts/").unwrap();
    ///user.doc_get_posts(&page);
    ///```
    pub fn doc_get_posts(&mut self, doc: &Document) -> TabunResult<Vec<Post>> {
        let mut ret = Vec::new();

        for p in doc.find(Name("article")).iter() {
            let post_id = try_to_parse!(hado!{
                el <- p.find(And(Name("div"),Class("vote-topic"))).first();
                attr <- el.attr("id");
                id_s <- attr.split('_').collect::<Vec<_>>().get(3);
                id_s.parse::<u32>().ok()
            });

            let post_title = try_to_parse!(p.find(And(Name("h1"),Class("topic-title"))).first()).text();

            let post_body = try_to_parse!(p.find(And(Name("div"),Class("topic-content"))).first()).inner_html();
            let post_body = post_body.trim();

            let post_date = try_to_parse!(p.find(And(Name("li"),Class("topic-info-date"))).find(Name("time")).first());
            let post_date = try_to_parse!(post_date.attr("datetime"));

            let post_tags = p.find(And(Name("a"),Attr("rel","tag"))).iter().fold(Vec::new(), |mut acc, x| {
                acc.push(x.text());
                acc
            });

            let cm_count = try_to_parse!(hado!{
                el <- p.find(And(Name("li"),Class("topic-info-comments"))).first();
                c_el <- el.find(Name("span")).first();
                c_el.text().parse::<u32>().ok() });

            let post_author = try_to_parse!(p.find(And(Name("div"),Class("topic-info")))
                                            .find(And(Name("a"),Attr("rel","author")))
                                            .first()).text();
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
    pub fn get_editable_post(&mut self, post_id: u32) -> TabunResult<EditablePost> {
        let doc = try!(self.get_document(&format!("/topic/edit/{}",post_id)));
        self.doc_get_editable_post(&doc)
    }

    pub fn doc_get_editable_post(&self, doc: &Document) -> TabunResult<EditablePost> {
        let title = try_to_parse!(doc.find(Attr("id","topic_title")).first());
        let title = try_to_parse!(title.attr("value")).to_string();

        let body = try_to_parse!(doc.find(Attr("id","topic_text")).first()).text();

        let tags = try_to_parse!(doc.find(Attr("id","topic_tags")).first());
        let tags = try_to_parse!(tags.attr("value"))
            .split(',').map(|x| x.to_string()).collect::<Vec<String>>();

        Ok(EditablePost{
            title:  title,
            body:   body,
            tags:   tags
        })
    }

    ///Получает EditablePost со страницы редактирования опроса
    ///(на момент написания документации варианты ответа не получаются,
    ///так как Табун всё равно не даёт их редактировать и нет смысла получать)
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_editable_poll(1111);
    ///```
    pub fn get_editable_poll(&mut self, post_id: u32) -> TabunResult<EditablePost> {
        let doc = try!(self.get_document(&format!("/question/edit/{}",post_id)));
        self.doc_get_editable_poll(&doc)
    }

    pub fn doc_get_editable_poll(&self, doc: &Document) -> TabunResult<EditablePost> {
        // TODO: всё-таки получить варианты ответов
        self.doc_get_editable_post(doc)
    }

    ///Получает пост, блог можно опустить (передать `None`), но лучше так не делать,
    ///дабы избежать дополнительных перенаправлений.
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_post("computers",157198);
    /// //или
    ///user.get_post("",157198);
    ///```
    pub fn get_post<'f, T: Into<Option<&'f str>>>(&mut self,blog_name: T,post_id: u32) -> TabunResult<Post>{
        let doc = match blog_name.into() {
            None    => try!(self.get_document(&format!("/blog/{}.html",post_id))),
            Some(x) => try!(self.get_document(&format!("/blog/{}/{}.html",x,post_id)))
        };
        self.doc_get_post(&doc)
    }

    pub fn doc_get_post(&mut self, doc: &Document) -> TabunResult<Post>{
        let post_id = try_to_parse!(hado!{
            el <- doc.find(And(Name("div"),Class("vote-topic"))).first();
            attr <- el.attr("id");
            id_s <- attr.split('_').collect::<Vec<_>>().get(3);
            id_s.parse::<u32>().ok()
        });

        let post_title = try_to_parse!(doc.find(And(Name("h1"),Class("topic-title"))).first()).text();

        let post_body = try_to_parse!(doc.find(And(Name("div"),Class("topic-content"))).first()).inner_html();
        let post_body = post_body.trim();

        let post_date = try_to_parse!(doc.find(And(Name("li"),Class("topic-info-date")))
                                      .find(Name("time"))
                                      .first());
        let post_date = try_to_parse!(post_date.attr("datetime"));

        let post_tags = doc.find(And(Name("a"),Attr("rel","tag"))).iter().fold(Vec::new(),|mut acc,t| {
            acc.push(t.text());
            acc
        });

        let cm_count = try_to_parse!(hado!{
            el <- doc.find(And(Name("span"),Attr("id","count-comments"))).first();
            el.text().parse::<u32>().ok()
        });

        let post_author = try_to_parse!(doc.find(And(Name("div"),Class("topic-info")))
                                        .find(And(Name("a"),Attr("rel","author")))
                                        .first()).text();

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
    pub fn edit_post(&mut self, post_id: u32, blog_id: u32, title: &str, body: &str, tags: &[String], forbid_comment: bool) -> TabunResult<u32> {
        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.to_owned();
        let forbid_comment = if forbid_comment { "1" } else { "0" };
        let tags = tags.iter().fold(String::new(), |acc, x| format!("{},{}", acc, x));

        let bd = vec![
            ("topic_type",            "topic"),
            ("blog_id",               &blog_id),
            ("topic_title",           title),
            ("topic_text",            body),
            ("topic_tags",            &tags),
            ("submit_topic_publish",  "Опубликовать"),
            ("security_ls_key",       &key),
            ("topic_forbid_comment",  &forbid_comment)
        ];

        let res = try!(self.post_multipart(&format!("/topic/edit/{}",post_id), &bd));

        let r = str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        parse_text_to_res!(regex => r"(\d+).html$", st => r, num => 1, typ => u32)

    }

    ///Редактирует опрос, возвращает его ID. Редактирование заголовка и
    ///вариантов ответа не реализованы, так как Табун всё равно не даёт их
    ///редактировать.
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("computers").unwrap();
    ///user.edit_poll(157198, blog_id, "Новый текст", &vec!["тэг"], false, true);
    ///```
    pub fn edit_poll(&mut self, post_id: u32, blog_id: u32, body: &str, tags: &[&str], forbid_comment: bool, publish: bool) -> TabunResult<u32> {
        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.to_owned();
        let forbid_comment = if forbid_comment { "1" } else { "0" };
        let tags = tags.iter().fold(String::new(), |acc, x| format!("{},{}", acc, x));

        let mut bd = vec![
            ("topic_type",            "question"),
            ("blog_id",               &blog_id),
            ("topic_title",           ""),
            ("topic_text",            body),
            ("topic_tags",            &tags),
            ("security_ls_key",       &key),
            ("topic_forbid_comment",  &forbid_comment)
        ];

        if publish {
            bd.push(("submit_topic_publish", "Опубликовать"));
        } else {
            bd.push(("submit_topic_save", "Сохранить в черновиках"));
        }

        let res = try!(self.post_multipart(&format!("/question/edit/{}",post_id), &bd));

        let r = str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        parse_text_to_res!(regex => r"(\d+).html$", st => r, num => 1, typ => u32)

    }

    ///Удаляет пост, и, так как табун ничего не возаращет по этому поводу,
    ///выдаёт `Ok(())` в случае удачи
    pub fn delete_post(&mut self, post_id: u32) -> TabunResult<()> {
        let url = format!("/topic/delete/{}/?security_ls_key={}", post_id ,&self.security_ls_key);
        match self.create_middle_req(&url)
            .header(Referer(format!("{}/blog/{}.html", HOST_URL, post_id)))
            .send().unwrap().status {
                hyper::Ok => Ok(()),
                x => Err(TabunError::NumError(x))
            }
    }

    ///Добавить пост в изранное или удалить его оттуда (true/false)
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.favourite_post(12345, true);
    ///```
    pub fn favourite_post(&mut self, id: u32, typ: bool) -> TabunResult<u32> {
        self.favourite(id, typ, false)
    }
}

#[cfg(test)]
mod test {
    use ::TClient;

    #[test]
    fn test_get_post() {
        let mut user = TClient::new(None,None).unwrap();
        match user.get_post("news",67052) {
            Ok(x)   => {
                assert_eq!(x.author, "Orhideous");
                assert_eq!(x.date, "2013-06-16T15:00:06+04:00");
                assert!(x.tags.contains(&"успех".to_string()))
            },
            Err(x)  => panic!(x)
        }
    }
}
