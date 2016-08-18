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
    pub fn get_comments(&mut self,url: &str) -> Result<HashMap<i64,Comment>,TabunError> {
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

        for comm in comments.find(Class("comment")).iter() {
            let parent = if comm.parent().unwrap().parent().unwrap().is(And(Name("div"),Class("comment-wrapper"))) {
                match comm.find(And(Name("li"),Class("vote"))).first() {
                    Some(x) => x.attr("id").unwrap().split('_').collect::<Vec<_>>()[3].parse::<i64>().unwrap(),
                    None => comm.attr("id").unwrap().split('_').collect::<Vec<_>>()[2].parse::<i64>().unwrap()
                }
            } else {
                0_i64
            };

            let post_id = if url == "/comments" {
                let url_regex = Regex::new(r"(\d+).html$").unwrap();
                let c = comm.find(Class("comment-path-topic"))
                    .first()
                    .unwrap();
                url_regex.captures(c.attr("href").unwrap())
                    .unwrap()
                    .at(1)
                    .unwrap()
                    .parse::<i32>()
                    .unwrap()
            } else {
                let url_regex = Regex::new(r"(\d+).html$").unwrap();
                url_regex.captures(url)
                    .unwrap()
                    .at(1)
                    .unwrap()
                    .parse::<i32>()
                    .unwrap()
            };

            let text = comm.find(And(Name("div"),Class("text"))).first().unwrap().inner_html();
            let text = text.as_str();

            let id = match comm.find(And(Name("li"),Class("vote"))).first() {
                Some(x) => x.attr("id").unwrap().split('_').collect::<Vec<_>>()[3].parse::<i64>().unwrap(),
                None => comm.attr("id").unwrap().split('_').collect::<Vec<_>>()[2].parse::<i64>().unwrap()
            };

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
                post_id:    post_id
            });
        }
        Ok(ret)
    }

    ///Оставить коммент к какому-нибудь посту, reply=0 - ответ на сам пост,
    ///иначе на чей-то коммент
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comment(1234,"Привет!", 0, libtabun::CommentType::Post);
    ///```
    pub fn comment(&mut self,post_id: i32, body : &str, reply: i32, typ: CommentType) -> Result<i64,TabunError>{
        use mdo::option::{bind};

        let id_regex = Regex::new("\"sCommentId\":(\\d+)").unwrap();
        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();

        let url = format!("/{}/ajaxaddcomment?security_ls_key={}&cmt_target_id={}&reply={}&comment_text={}"
                          , match typ { CommentType::Post => "blog", CommentType::Talk => "talk" }
                          , self.security_ls_key
                          , post_id
                          , reply
                          , body);

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
            ret r.parse::<i64>().ok()
        ).unwrap())
    }

    ///Подписаться/отписаться от комментариев к посту.
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comments_subscribe(157198,false);
    ///```
    pub fn comments_subscribe(&mut self, post_id: i32, subscribed: bool) {
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
