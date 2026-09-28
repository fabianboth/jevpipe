from concurrent.futures import ThreadPoolExecutor

import httpx

from bench import dataset, labels, pool, store

_FETCHES = 16


def prepare(suite: store.Suite) -> None:
    ratings = dataset.ratings_of(dataset.download(), suite)
    texts = [text for text, _ in ratings]
    queries = (
        dataset.split(texts)
        if suite.primary
        else dataset.all_test(dataset.load(store.PYTHON), texts)
    )
    urls = sorted({url for _, url in ratings})
    with httpx.Client(timeout=60) as client, ThreadPoolExecutor(_FETCHES) as executor:
        codes = dict(zip(urls, executor.map(pool.fetcher(client), urls), strict=True))
    snippets, texts_of_pool = pool.build(urls, codes.get, suite)
    pool.write_files(texts_of_pool, suite)
    pool.save(snippets, suite)
    dataset.save(queries, suite)
    labels.save_experts(labels.experts(queries, snippets, ratings), suite)
    print(
        f"prepare: {suite.label}: {len(snippets.snippets)} snippets, "
        f"{len(snippets.missing)} missing, {len(queries.of_split('dev'))} dev and "
        f"{len(queries.of_split('test'))} test queries"
    )
