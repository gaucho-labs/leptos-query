use std::str::FromStr;

use leptos::*;
use leptos_meta::Html;
use leptos_query::query_persister::{IndexedDbPersister, LocalStoragePersister};
use leptos_query::use_query_client;
use leptos_use::storage::use_local_storage;
use leptos_use::utils::FromToStringCodec;

use crate::components::switch::Switch;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
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
                        <div class="absolute bottom-4 flex flex-col items-start gap-2">
                            <ThemeToggle/>
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
fn ThemeToggle() -> impl IntoView {
    let current_theme = use_theme();
    let is_dark = Signal::derive(move || current_theme.get() == Theme::Dark);

    view! {
        // <Html class=move || if is_dark.get() { "dark" } else { "" }/>
        <label
            for="dark-mode-toggle"
            class="text-xs font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70"
        >
            Dark Mode
        </label>
        <Switch
            enabled=is_dark
            on_click=Callback::new(move |_| {
                let new_theme = if is_dark.get() { Theme::Light } else { Theme::Dark };
                current_theme.set(new_theme);
            })

            attr:id="dark-mode-toggle"
        />
    }
}

#[component]
fn SelectPersister() -> impl IntoView {
    let (persister, set_persister, _) =
        use_local_storage::<Persister, FromToStringCodec>(Persister::None);

    create_effect(move |_| set_persister(persister.get_untracked()));

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
            class="form-select border-border border text-xs rounded-md block py-1 pr-8 bg-input text-input-foreground line-clamp-1 focus:border-primary focus:ring focus:ring-primary/50 transition-colors"
            prop:value=move || persister.get().as_str()
            on:change=move |ev| {
                let new_value = event_target_value(&ev);
                let option = FromStr::from_str(&new_value).unwrap();
                set_persister(option);
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Persister {
    LocalStorage,
    IndexDB,
    None,
}

impl Persister {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LocalStorage => "LocalStorage",
            Self::IndexDB => "IndexDB",
            Self::None => "None",
        }
    }
}

impl std::fmt::Display for Persister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Persister {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s {
            "LocalStorage" => Self::LocalStorage,
            "IndexDB" => Self::IndexDB,
            _ => Self::None,
        };

        Ok(result)
    }
}

impl AsRef<str> for Persister {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Default for Persister {
    fn default() -> Self {
        Self::None
    }
}
