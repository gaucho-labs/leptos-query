use std::rc::Rc;

use crate::repo::*;

use leptos::*;
use leptos_query::*;
use leptos_router::{ActionForm, MultiActionForm};
use serde::{Deserialize, Serialize};

#[component]
pub fn TodoPage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center w-full h-full font-medium">
            <h1 class="font-semibold ml-3 text-lg">Leptos Query Todos</h1>
            <AddTodoEntry/>
            <TodoList/>
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
                            view! { <TodoListItem todo=todo/> }
                        }
                    />

                </Show>
            </ul>
        </Transition>
    }
}

#[component]
fn TodoListItem(todo: Todo) -> impl IntoView {
    let delete_todo = create_server_action::<DeleteTodo>();

    // Optimistic update.
    create_effect({
        let todo_query = todo_query();
        let all_todos = all_todos_query();
        move |_| {
            if let Some(Some(input)) = delete_todo.input().try_get() {
                let id = input.id;
                all_todos.cancel_query(AllTodos);
                todo_query.set_query_data(TodoId(id), Ok(None));
                all_todos.update_query_data_mut(AllTodos, |todos| {
                    if let Ok(todos) = todos {
                        todos.retain(|t| t.id != id);
                    }
                });
            }
        }
    });

    // Invalidate queries on successful delete.
    create_effect({
        let todo_query = todo_query();
        let all_todos = all_todos_query();

        move |_| {
            if let Some(_) = delete_todo.value().get() {
                all_todos.invalidate_query(AllTodos);
                todo_query.invalidate_query(TodoId(todo.id));
            }
        }
    });

    view! {
        <li class="flex items-center justify-between w-full p-2 m-2 bg-gray-100 rounded-md">
            <h2 class="font-semibold">{todo.title}</h2>
            <ActionForm action=delete_todo>
                <input type="hidden" name="id" value=todo.id/>
                <button type="submit" value="Delete">
                    Delete
                </button>
            </ActionForm>
        </li>
    }
}

#[component]
fn AddTodoEntry() -> impl IntoView {
    let todo_query = todo_query();
    let all_todos = all_todos_query();

    let add_todo = create_server_multi_action_with_callbacks::<AddTodo>(
        |_| {},
        move |(_, result)| {
            if let Ok(todo) = result {
                all_todos.cancel_query(AllTodos);
                all_todos.update_query_data_mut(AllTodos, |todos| {
                    if let Ok(todos) = todos {
                        todos.push(todo.clone());
                    }
                });

                todo_query.set_query_data(TodoId(todo.id), Ok(Some(todo.clone())));
                todo_query.invalidate_query(TodoId(todo.id));
                all_todos.invalidate_query(AllTodos);
            }
        },
    );

    view! {
        <MultiActionForm action=add_todo class="flex flex-col items-start gap-2">
            <input type="text" name="create[title]"/>
            <input type="text" name="create[description]"/>
            <input type="submit" autocomplete="off" value="Add"/>
        </MultiActionForm>
    }
}

pub fn create_server_multi_action_with_callbacks<S>(
    on_invoke: impl Fn(&S) + 'static,
    on_settled: impl Fn((&S, &Result<S::Output, ServerFnError<S::Error>>)) + 'static,
) -> MultiAction<S, Result<S::Output, ServerFnError<S::Error>>>
where
    S: Clone + server_fn::ServerFn,
{
    let on_invoke = Rc::new(on_invoke);
    let on_settled = Rc::new(on_settled);
    #[cfg(feature = "ssr")]
    let c = {
        move |args: &S| {
            let args = args.clone();
            let on_invoke = on_invoke.clone();
            let on_settled = on_settled.clone();
            async move {
                on_invoke(&args);
                let result = S::run_body(args.clone()).await;
                on_settled((&args, &result));
                result
            }
        }
    };
    #[cfg(not(feature = "ssr"))]
    let c = {
        move |args: &S| {
            let args = args.clone();
            let on_invoke = on_invoke.clone();
            let on_settled = on_settled.clone();
            async move {
                on_invoke(&args);
                let result = S::run_on_client(args.clone()).await;
                on_settled((&args, &result));
                result
            }
        }
    };

    create_multi_action(c).using_server_fn::<S>()
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
