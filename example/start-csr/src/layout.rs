use leptos::*;

#[component]
pub fn Layout(children: Children) -> impl IntoView {
    view! {
        <div class="relative flex min-h-screen flex-col bg-background">
            <div class="container flex-1 items-start md:grid md:grid-cols-[220px_minmax(0,1fr)] md:gap-6 lg:grid-cols-[240px_minmax(0,1fr)] lg:gap-10">
                <aside class="h-full w-full shrink-0 border-r">
                    <div class="relative overflow-hidden h-full py-6 pr-6 lg:py-8">
                        <div class="grid grid-flow-row auto-rows-max text-sm">
                            <SidebarLink href="/">Home</SidebarLink>
                            <SidebarLink href="/single">Single Query</SidebarLink>
                        </div>
                    </div>
                </aside>
                <main class="relative py-6 lg:py-8">{children()}</main>
            </div>
        </div>
    }
}

#[component]
pub fn SidebarLink(#[prop(into)] href: String, children: Children) -> impl IntoView {
    view! {
        <a
            href=href
            class="group flex w-full items-center rounded-md border border-transparent px-2 py-1 hover:underline font-medium text-foreground"
        >
            {children()}
        </a>
    }
}
