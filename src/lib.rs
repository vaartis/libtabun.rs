extern crate hyper;
extern crate select;
extern crate regex;
extern crate cookie;
extern crate time;
extern crate multipart;
#[macro_use] extern crate mdo;

use std::fmt::Display;
use std::str::FromStr;

use regex::Regex;

use std::collections::HashMap;

use hyper::client::Client;
use hyper::client::request::Request;
use hyper::header::{SetCookie,Cookie};
use hyper::status::StatusCode;

use multipart::client::Multipart;

use std::io::Read;

use select::document::Document;
use select::predicate::{Class, Name, And, Attr};

use cookie::CookieJar;

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

#[derive(Debug)]
pub enum TabunError {
    ///На случай `Hacking attempt!`
    HackingAttempt,
    ///Ошибка с названием и описанием,
    ///обычно соответствует табуновским
    ///всплывающим сообщениям
    ///TODO: сделать их читаемыми
    Error(String,String),
    ///Ошибка с номером, вроде 404 и 403
    NumError(StatusCode)
}

impl From<StatusCode> for TabunError {
    fn from(x: StatusCode) -> Self {
        TabunError::NumError(x)
    }
}

///Клиент табуна
pub struct TClient<'a> {
    pub name:               String,
    pub security_ls_key:    String,
    client:                 Client,
    cookies:                CookieJar<'a>,
}

#[derive(Debug,Clone)]
pub struct Comment {
    pub body:   String,
    pub id:     i64,
    pub author: String,
    pub date:   String,
    pub votes:  i32,
    pub parent: i64,
}

#[derive(Debug,Clone)]
pub struct Post {
    pub title:          String,
    pub body:           String,
    pub date:           String,
    pub tags:           Vec<String>,
    pub comments_count: i32,
    pub author:         String,
    pub id:             i32,
}

#[derive(Debug,Clone)]
pub struct EditablePost {
    pub title:          String,
    pub body:           String,
    pub tags:           Vec<String>,
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Comment({},\"{}\",\"{}\")", self.id, self.author, self.body)
    }
}

impl Display for Post {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Post({},\"{}\",\"{}\")", self.id, self.author, self.body)
    }
}

///URL сайта. Ибо по идее может работать и с другими штуками ня лайвстрите
pub const HOST_URL: &'static str = "https://tabun.everypony.ru";

impl<'a> TClient<'a> {

    ///Входит на табунчик и сохраняет LIVESTREET_SECURITY_KEY,
    ///если логин или пароль == "" - анонимус.
    ///
    ///# Examples
    ///```no_run
    ///let mut user = libtabun::TClient::new("логин","пароль");
    ///```
    pub fn new(login: &str, pass: &str) -> Result<TClient<'a>,TabunError> {
        if login == "" || pass == "" {
            return Ok(TClient{
                name:               String::new(),
                security_ls_key:    String::new(),
                client:             Client::new(),
                cookies:            CookieJar::new(time::now().to_timespec().sec.to_string().as_bytes()),
            });
        }

        let mut user = TClient::new("","").unwrap();

        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();
        let hacking_regex = Regex::new("Hacking").unwrap();

        let ls_key_regex = Regex::new(r"LIVESTREET_SECURITY_KEY = '(.+)'").unwrap();

        let page = try!(user.get(&"/login".to_owned()));

        let page_html = page.find(Name("html")).first().unwrap().html();

        user.security_ls_key = ls_key_regex.captures(&page_html).unwrap().at(1).unwrap().to_owned();

        let added_url = format!("/login/ajax-login?login={}&password={}&security_ls_key={}"
                                , login, pass, user.security_ls_key);

        let res = try!(user.get(&added_url));

        let res = res.nth(0).unwrap().text();
        let res = res.as_str();


        if hacking_regex.is_match(res) {
            Err(TabunError::HackingAttempt)
        } else if err_regex.is_match(res) {
            let err = err_regex.captures(res).unwrap();
            Err(TabunError::Error(err.at(1).unwrap().to_owned(),err.at(2).unwrap().to_owned()))
        } else {
            let page = try!(user.get(&"".to_owned()));

            user.name = page.find(Class("username")).first().unwrap().text();

            Ok(user)
        }
    }

    fn get(&mut self,url: &String) -> Result<Document,StatusCode>{
        let full_url = format!("{}{}", HOST_URL, url);

        let mut res = self.client.get(
            &full_url)
            .header(Cookie::from_cookie_jar(&self.cookies))
            .send()
            .unwrap();

        if res.status != hyper::Ok { return Err(res.status) }

        let mut buf = String::new();
        res.read_to_string(&mut buf).unwrap();

        let cookie = if res.headers.has::<SetCookie>() {
            Some(res.headers.get::<SetCookie>().unwrap())
        } else {
            None
        };

        if let Some(_) = cookie {
            cookie.unwrap().apply_to_cookie_jar(&mut self.cookies);
        }

        Ok(Document::from(&*buf))
    }

    fn multipart(&mut self,url: &str, bd: HashMap<&str,&str>) -> Result<hyper::client::Response,StatusCode> {
        let url = format!("{}{}", HOST_URL, url);
        let mut request = Request::new(hyper::method::Method::Post,
                               hyper::Url::from_str(&url).unwrap()).unwrap();
        request.headers_mut().set(Cookie::from_cookie_jar(&self.cookies));

        let mut req = Multipart::from_request(request).unwrap();

        for (param,val) in bd {
            let _ = req.write_text(param,val);
        }

        let res = req.send().unwrap();

        if res.status != hyper::Ok && res.status != hyper::status::StatusCode::MovedPermanently { return Err(res.status) }

        Ok(res)
    }

    ///Оставить коммент к какому-нибудь посту, reply=0 - ответ на сам пост,
    ///иначе на чей-то коммент
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comment(1234,"Привет!",0);
    ///```
    pub fn comment(&mut self,post_id: i32, body : &str, reply: i32) -> Result<i64,TabunError>{
        use mdo::option::{bind};

        let id_regex = Regex::new("\"sCommentId\":(\\d+)").unwrap();
        let err_regex = Regex::new("\"sMsgTitle\":\"(.+)\",\"sMsg\":\"(.+?)\"").unwrap();

        let url = format!("/blog/ajaxaddcomment?security_ls_key={}&cmt_target_id={}&reply={}&comment_text={}"
                          , self.security_ls_key,post_id,reply,body);

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

    ///Получить комменты из некоторого поста
    ///в виде HashMap ID-Коммент. Если блог указан как ""
    ///и пост указан как 0, то получает из `/comments/`
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_comments("lighthouse",157807);
    ///```
    pub fn get_comments(&mut self,blog: &str, post_id: i32) -> Result<HashMap<i64,Comment>,TabunError> {
        let mut ret = HashMap::new();

        let ref url = if blog == "" && post_id == 0 {
            "/comments".to_owned()
        } else {
            format!("/blog/{}/{}.html", blog, post_id)
        };

        let page = try!(self.get(url));

        let comments = page.find(And(Name("div"),Class("comments")));

        for comm in comments.find(Class("comment")).iter() {
            let mut parent = 0i64;
            if comm.parent().unwrap().parent().unwrap().is(And(Name("div"),Class("comment-wrapper"))) {
                let p = comm.find(And(Name("li"),Class("vote"))).first().unwrap();
                parent = p.attr("id").unwrap().split("_").collect::<Vec<_>>()[3].parse::<i64>().unwrap();
            }

            let text = comm.find(And(Name("div"),Class("text"))).first().unwrap().inner_html();
            let text = text.as_str();

            let id = comm.find(And(Name("li"),Class("vote"))).first().unwrap();
            let id = id.attr("id").unwrap().split("_").collect::<Vec<_>>()[3].parse::<i64>().unwrap();

            let author = comm.find(And(Name("li"),Class("comment-author")))
                .find(Name("a"))
                .first()
                .unwrap();
            let author = author.attr("href").unwrap().split("/").collect::<Vec<_>>()[4];

            let date = comm.find(Name("time")).first().unwrap();
            let date = date.attr("datetime").unwrap();

            let votes = comm.find(And(Name("span"),Class("vote-count")))
                .first()
                .unwrap()
                .text().parse::<i32>().unwrap();
            ret.insert(id,Comment{
                body:   text.to_owned(),
                id:     id,
                author: author.to_owned(),
                date:   date.to_owned(),
                votes:  votes,
                parent: parent,
            });
        }
        return Ok(ret);
    }

    ///Получает ID блога по его имени
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("lighthouse").unwrap();
    ///assert_eq!(blog_id,15558);
    ///```
    pub fn get_blog_id(&mut self,name: &str) -> Result<i32,TabunError> {
        use mdo::option::{bind,ret};

        let url = format!("/blog/{}", name);
        let page = try!(self.get(&url));

        Ok(mdo!(
            x =<< page.find(And(Name("div"),Class("vote-item"))).first();
            x =<< x.find(Name("span")).first();
            x =<< x.attr("id");
            x =<< x.split("_").collect::<Vec<_>>().last();
            x =<< x.parse::<i32>().ok();
            ret ret(x)
        ).unwrap())
    }

    ///Создаёт пост в указанном блоге
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("computers").unwrap();
    ///user.add_post(blog_id,"Название поста","Текст поста",vec!["тэг раз","тэг два"]);
    ///```
    pub fn add_post(&mut self, blog_id: i32, title: &str, body: &str, tags: Vec<&str>) -> Result<i32,TabunError> {
        use mdo::option::{bind};

        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.clone();
        let mut rtags = String::new();
        for i in tags {
            rtags += &format!("{},", i);
        }

        let bd = map![
            "topic_type"            =>  "topic",
            "blog_id"               =>  &blog_id,
            "topic_title"           =>  title,
            "topic_text"            =>  body,
            "topic_tags"            =>  &rtags,
            "submit_topic_publish"  =>  "Опубликовать",
            "security_ls_key"       =>  &key
        ];

        let res = try!(self.multipart("/topic/add",bd));

        let r = std::str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        Ok(mdo!(
            regex       =<< Regex::new(r"(\d+).html$").ok();
            captures    =<< regex.captures(r);
            r           =<< captures.at(1);
            ret r.parse::<i32>().ok()
        ).unwrap())
    }

    ///Редактирует пост
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///let blog_id = user.get_blog_id("computers").unwrap();
    ///user.edit_post(157198,blog_id,"Новое название", "Новый текст", vec!["тэг".to_string()],false);
    ///```
    pub fn edit_post(&mut self, post_id: i32, blog_id: i32, title: &str, body: &str, tags: Vec<String>, forbid_comment: bool) -> Result<i32,TabunError> {
        use mdo::option::{bind};

        let blog_id = blog_id.to_string();
        let key = self.security_ls_key.clone();
        let forbid_comment = if forbid_comment == true { "1" } else { "0" };
        let mut rtags = String::new();
        for i in tags {
            rtags += &format!("{},", i);
        }

        let bd = map![
            "topic_type"            =>  "topic",
            "blog_id"               =>  &blog_id,
            "topic_title"           =>  title,
            "topic_text"            =>  body,
            "topic_tags"            =>  &rtags,
            "submit_topic_publish"  =>  "Опубликовать",
            "security_ls_key"       =>  &key,
            "topic_forbid_comment"  =>  &forbid_comment
        ];

        let res = try!(self.multipart(&format!("/topic/edit/{}",post_id), bd));

        let r = std::str::from_utf8(&res.headers.get_raw("location").unwrap()[0]).unwrap();

        Ok(mdo!(
            regex       =<< Regex::new(r"(\d+).html$").ok();
            captures    =<< regex.captures(r);
            r           =<< captures.at(1);
            ret r.parse::<i32>().ok()
        ).unwrap())
    }

    ///Получает посты из блога
    ///
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.get_posts("lighthouse",1);
    ///```
    pub fn get_posts(&mut self, blog_name: &str, page: i32) -> Result<Vec<Post>,TabunError>{
       let res = try!(self.get(&format!("/blog/{}/page{}", blog_name, page)));
       let mut ret = Vec::new();

       for p in res.find(Name("article")).iter() {
        let post_id = p.find(And(Name("div"),Class("vote-topic")))
               .first()
               .unwrap()
               .attr("id")
               .unwrap()
               .split("_").collect::<Vec<_>>()[3].parse::<i32>().unwrap();

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
            .parse::<i32>().unwrap();

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
    pub fn get_editable_post(&mut self, post_id: i32) -> Result<EditablePost,TabunError> {
        let res = try!(self.get(&format!("/topic/edit/{}",post_id)));

        let title = res.find(Attr("id","topic_title")).first().unwrap();
        let title = title.attr("value").unwrap().to_string();

        let tags = res.find(Attr("id","topic_tags")).first().unwrap();
        let tags = tags.attr("value").unwrap();
        let tags = tags.split(",").map(|x| x.to_string()).collect::<Vec<String>>();

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
    pub fn get_post(&mut self,blog_name: &str,post_id: i32) -> Result<Post,TabunError>{
        let res = if blog_name == "" {
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
            .parse::<i32>()
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

    ///Подписаться/отписаться от комментариев к посту.
    ///# Examples
    ///```no_run
    ///# let mut user = libtabun::TClient::new("логин","пароль").unwrap();
    ///user.comments_subscribe(157198,false);
    ///```
    pub fn comments_subscribe(&mut self, post_id: i32, subscribed: bool) {
        let subscribed = if subscribed == true { "1" } else { "0" };

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
