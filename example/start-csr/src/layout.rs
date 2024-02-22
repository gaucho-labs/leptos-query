use leptos::*;
use leptos_query::query_persister::{IndexedDbPersister, LocalStoragePersister};
use leptos_query::use_query_client;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    provide_context(AppState::default());

    view! {
        <div class="relative flex min-h-screen flex-col bg-background">
            <div class="flex-1 items-start grid grid-cols-[180px_minmax(0,1fr)] md:gap-6 lg:grid-cols-[200px_minmax(0,1fr)] lg:gap-10">
                <aside class="h-full w-full shrink-0 border-r">
                    <div class="relative overflow-hidden h-full py-6 pr-6 lg:py-8 px-2 md:px-4">
                        <SidebarLink href="/">Home</SidebarLink>
                        <div class="py-2"></div>
                        <h4 class="rounded-md px-2 py-1 text-base font-semibold">Examples</h4>
                        <div class="grid grid-flow-row auto-rows-max text-sm">
                            <SidebarLink href="/single">Single Query</SidebarLink>
                            <SidebarLink href="/todos">Optimistic Update</SidebarLink>
                        </div>
                        <div class="absolute bottom-4">
                            <SelectPersister/>
                        </div>
                    </div>
                </aside>
                <main class="container relative py-6 lg:py-8">{children()}</main>
            </div>
        </div>
    }
}

#[component]
fn SelectPersister() -> impl IntoView {
    let state = use_app_state();
    let persister = state.persister;

    create_effect(move |_| {
        let client = use_query_client();
        let persister = persister.get();

        match persister {
            Persister::LocalStorage => {
                client.remove_persister();
                client.add_persister(LocalStoragePersister);
            }
            Persister::IndexDB => {
                client.remove_persister();
                client.add_persister(IndexedDbPersister::default());
            }
            Persister::None => {
                client.remove_persister();
            }
        }
    });

    view! {
        <label
            for="query-persister"
            class="text-xs font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
        >
            Query Persister
        </label>
        <select
            id="query-persister"
            class="form-select border-border border text-xs rounded-md block py-1 px-4 bg-input text-input-foreground line-clamp-1"
            value=move || persister.get().as_str()
            on:change=move |ev| {
                let new_value = event_target_value(&ev);
                let option = Persister::from_string(&new_value);
                persister.set(option);
            }
        >

            <option value=Persister::None.as_str()>None</option>
            <option value=Persister::IndexDB.as_str()>Indexed DB</option>
            <option value=Persister::LocalStorage.as_str()>Local Storage</option>
        </select>
    }
}

#[component]
pub fn SidebarLink(#[prop(into)] href: String, children: Children) -> impl IntoView {
    view! {
        <a
            href=href
            class="group flex w-full items-center rounded-md border border-transparent px-2 py-1 hover:underline font-medium text-foreground/80"
        >
            {children()}
        </a>
    }
}

#[derive(Clone)]
pub struct AppState {
    persister: RwSignal<Persister>,
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>().expect("Missing AppState")
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            persister: create_rw_signal(Persister::None),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Persister {
    LocalStorage,
    IndexDB,
    None,
}

impl Persister {
    pub fn from_string(s: &str) -> Self {
        match s {
            "LocalStorage" => Self::LocalStorage,
            "IndexDB" => Self::IndexDB,
            _ => Self::None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalStorage => "LocalStorage",
            Self::IndexDB => "IndexDB",
            Self::None => "None",
        }
    }
}
