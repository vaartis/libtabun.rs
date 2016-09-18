[![Build Status](https://travis-ci.org/TyanNN/libtabun.rs.svg?branch=master)](https://travis-ci.org/TyanNN/libtabun.rs)
# libtabun.rs
API для tabun.everypony.ru

# Установка

```toml
[dependencies]
libtabun = { git = "https://github.com/TyanNN/libtabun.rs" }
```

```bash
cargo build
```
# Документация

Можно почитать [тут](https://kotobank.ch/~easy/tabun_docs/libtabun/) или собрать самому:

```bash
cargo doc
```

# Roadmap
- [x] Логин
- [x] Читать
  - [x] Посты
  - [x] Комменты
  - [x] Личные сообщения
  - [x] Инфу о юзерах
- [x] Создавать
  - [x] Комменты
	- [x] В личных сообщениях
  - [x] Посты
    - [x] Редактирование
    - [x] Удаление
  - [x] Личные сообщения
- [ ] Опросы
- [ ] Инвайты
- [x] Избранное
  - [x] Посты
  - [x] Комменты
- [x] Активность из /comments
- [x] Загрузка картинок по ссылке
