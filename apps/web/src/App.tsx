import { useEffect, useMemo, useState } from 'react';
import { fetchTopPosts, type HackerNewsItem } from './hn';

type Status = 'idle' | 'loading' | 'ready' | 'error';

export default function App() {
  const [posts, setPosts] = useState<HackerNewsItem[]>([]);
  const [status, setStatus] = useState<Status>('idle');
  const [message, setMessage] = useState<string>('');

  useEffect(() => {
    const run = async () => {
      setStatus('loading');
      try {
        const result = await fetchTopPosts(20);
        setPosts(result);
        setStatus('ready');
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err);
        setMessage(error);
        setStatus('error');
      }
    };

    run();
  }, []);

  const content = useMemo(() => {
    if (status === 'loading' || status === 'idle') {
      return <p className="loading">Loading top stories…</p>;
    }

    if (status === 'error') {
      return (
        <div className="error">
          <p>Could not fetch Hacker News stories.</p>
          <code>{message}</code>
        </div>
      );
    }

    return (
      <ul>
        {posts.map((post) => (
          <li key={post.id} className="card">
            <a href={post.url ?? `https://news.ycombinator.com/item?id=${post.id}`} target="_blank" rel="noreferrer">
              {post.title}
            </a>
            <p>
              {post.score} points • {post.by}
            </p>
          </li>
        ))}
      </ul>
    );
  }, [message, posts, status]);

  return (
    <main>
      <header>
        <h1>Hacker News — Multi-platform</h1>
        <p>Shared Rust WebAssembly fetching the top stories.</p>
      </header>
      {content}
    </main>
  );
}
