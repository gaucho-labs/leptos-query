use leptos::*;

#[component]
pub fn Introduction() -> impl IntoView {
    view! {
        <article
        class=" p-4 prose dark:prose-invert max-w-[75ch] mx-auto lg:prose-lg prose-zinc
                prose-p:text-foreground prose-headings:text-foreground prose-lead:text-foreground prose-li:text-foreground prose-ul:text-foreground prose-ol:text-foreground
                prose-hr:border-border prose-hr:my-2
                prose-h1:my-1 lg:prose-h1:my-1
                prose-h2:my-1 lg:prose-h2:my-1
            ">
            <h1>Why Leptos Query?</h1>
            <hr></hr>
            <strong>What is a Query? What is async state?</strong>
            <p>Broadly, a Query is an async request for data, bound to a unique key. You often (but not always) "don't" own that state on the client.
            </p>
            <p>
                "We'll" call this state <code>Async State</code>.
                Here are some common properties:
            </p>
            <ul>
                <li>You do not control or own the "source of truth" on the client</li>
                <li>Requires async APIs for fetching data</li>
                <li>
                    Implies shared ownership and can be changed by other people without your knowledge
                </li>
                <li>
                    Can potentially become "out of date" in your apps if "you're" not careful
                </li>
            </ul>
            <p>
                Very often in programming highly dynamic web apps, you end up creating a client-side state machine to keep some
                <code>Async State</code>
                in sync with the actual source of truth (usually the server/database or the browser).
            </p>
            <p>
                This helps make your app "feel" more responsive, with instant updates and the like.
            </p>
            <h4> But there are issues... </h4>
            <ul>
                <li>"It's" a ton of work to get right</li>
                <li>
                    "It's" incredibly easy to get wrong (have incorrect state, requiring a page refresh)
                </li>
                <li>
                    It "doesn't" scale well. The more state you have, the more complex it gets
                </li>
            </ul>
            <h4>Here are some of the common challenges</h4>
            <ul>
                <li>Knowing when data is "out of date"</li>
                <li>Updating "out of date" data in the background</li>
                <li>No duplicate fetches for same data</li>
                <li>Configurable cache lifetimes</li>
                <li>Invalidation</li>
                <li>Cancellation</li>
                <li>Managing memory and garbage collection</li>
                <li>Updates (set to new value, update mut existing value, etc.)</li>
                <li>Client side persistence (local storage, indexed db, etc.)</li>
            </ul>
            <p>"Here's" where a Leptos Query comes in.</p>
            <h2>Leptos Query to the rescue</h2>
            <hr></hr>
            <p>
                Leptos Query is a client-side Query Manager, helping you manage
                <code>Async State</code> for highly dynamic and interactive Leptos web apps, which handles all of the above problems.
            </p>
            Practically speaking, by using Leptos Query, you will likely:
            <ul>
                <li>Reduce your code complexity and improve code maintainability by removing a lot of state management from your client side code</li>
                <li>Improve the user experience by improving the speed and responsiveness of your app</li>
                <li>Have a nicer debugging experience, with the provided Devtools</li>
            </ul>
        </article>
    }
}
