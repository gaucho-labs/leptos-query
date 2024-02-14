use crate::repo::*;

use leptos::*;
use leptos_query::*;
use serde::{Deserialize, Serialize};

#[component]
pub fn TodoPage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center w-full h-full font-medium">
            <h1 class="font-semibold ml-3 text-lg">Leptos Query Todos</h1>
            <TodoList />
        </div>
    }
}

#[component]
pub fn TodoList() -> impl IntoView {
    let result = all_todos_query().use_query(|| AllTodos);
    let todos = Signal::derive(move || result.data.get().and_then(|r| r.ok()).unwrap_or_default());

    view! {
        <Transition fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            <ul>
                <Show
                    when=move || !todos.get().is_empty()
                    fallback=|| {
                        view! { <p>"No todos"</p> }
                    }
                >

                    <For
                        each=todos
                        key=|todo| todo.id
                        children=move |todo| {
                            view! {
                                <TodoListItem todo=todo />
                            }
                        }
                    />

                </Show>
            </ul>
        </Transition>
    }
}

#[component]
fn TodoListItem(todo: Todo) -> impl IntoView {
    view! {
        <li class="flex items-center justify-between w-full p-2 m-2 bg-gray-100 rounded-md">
            <div>
                <h2 class="font-semibold">{todo.title}</h2>
            </div>
        </li>
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct AllTodos;

fn all_todos_query() -> QueryScope<AllTodos, Result<Vec<Todo>, ServerFnError>> {
    create_query(|_| get_todos(), QueryOptions::default())
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
struct TodoId(u16);
fn todo_query() -> QueryScope<TodoId, Result<Option<Todo>, ServerFnError>> {
    create_query(|id: TodoId| get_todo(id.0), QueryOptions::default())
}
