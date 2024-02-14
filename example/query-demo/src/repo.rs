use leptos::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::{Connection, SqliteConnection};

#[cfg(feature = "ssr")]
pub async fn db() -> Result<SqliteConnection, ServerFnError> {
    SqliteConnection::connect("sqlite:Todos.db")
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Todo {
    pub id: u16,
    pub title: String,
    pub description: String,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTodo {
    pub title: String,
    pub description: String,
}

#[server(AddTodo, "/api")]
pub async fn add_todo(create: CreateTodo) -> Result<Todo, ServerFnError> {
    let mut conn = db().await?;
    let result = sqlx::query("INSERT INTO todos (title, description, done) VALUES (?, ?, false)")
        .bind(&create.title)
        .bind(&create.description)
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

    let todo = Todo {
        id: result as u16,
        title: create.title,
        description: create.description,
        done: false,
    };

    Ok(todo)
}

#[server(GetTodos, "/api")]
pub async fn get_todos() -> Result<Vec<Todo>, ServerFnError> {
    let mut conn = db().await?;
    let todos = sqlx::query_as::<_, Todo>("SELECT * FROM todos")
        .fetch_all(&mut conn)
        .await?;

    Ok(todos)
}

#[server(GetTodo, "/api")]
pub async fn get_todo(id: u16) -> Result<Option<Todo>, ServerFnError> {
    let mut conn = db().await?;
    let todo = sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE id = ?")
        .bind(&id)
        .fetch_optional(&mut conn)
        .await?;

    Ok(todo)
}

#[server(UpdateTodo, "/api")]
pub async fn update_todo(todo: Todo) -> Result<Option<Todo>, ServerFnError> {
    let mut conn = db().await?;
    let result = sqlx::query("UPDATE todos SET title = ?, description = ?, done = ? WHERE id = ?")
        .bind(&todo.title)
        .bind(&todo.description)
        .bind(&todo.done)
        .bind(&todo.id)
        .execute(&mut conn)
        .await?;

    if result.rows_affected() == 0 {
        Ok(None)
    } else {
        Ok(Some(todo))
    }
}

#[server(DeleteTodo, "/api")]
pub async fn delete_todo(id: u16) -> Result<bool, ServerFnError> {
    let mut conn = db().await?;
    let result = sqlx::query("DELETE FROM todos WHERE id = ?")
        .bind(&id)
        .execute(&mut conn)
        .await?;

    if result.rows_affected() == 0 {
        Ok(false)
    } else {
        Ok(true)
    }
}
