from concurrent.futures import ThreadPoolExecutor

import httpx

from bench import dataset, labels, pool, store
from bench.limits import StageError

_FETCHES = 16


def prepare(suite: store.Suite) -> None:
    if not suite.primary and not (store.PYTHON.results / "split.json").is_file():
        message = "prepare Python first: the other languages reuse its search ids"
        raise StageError(message)
    with httpx.Client(timeout=60) as client, ThreadPoolExecutor(_FETCHES) as executor:
        ratings = dataset.ratings_of(dataset.annotations(client), suite)
        urls = sorted({url for _, url in ratings})
        codes = dict(zip(urls, executor.map(pool.fetcher(client), urls), strict=True))
    texts = [text for text, _ in ratings]
    queries = (
        dataset.split(texts)
        if suite.primary
        else dataset.all_test(dataset.load(store.PYTHON), texts)
    )
    snippets, texts_of_pool = pool.build(urls, codes.get, suite)
    _keep_runs_valid(snippets, suite)
    pool.write_files(texts_of_pool, suite)
    pool.save(snippets, suite)
    dataset.save(queries, suite)
    labels.save_experts(labels.experts(queries, snippets, ratings), suite)
    print(
        f"prepare: {suite.label}: {len(snippets.snippets)} snippets, "
        f"{len(snippets.missing)} missing, {len(queries.of_split('dev'))} dev and "
        f"{len(queries.of_split('test'))} test queries"
    )


def _keep_runs_valid(snippets: pool.Pool, suite: store.Suite) -> None:
    if not any(suite.runs.glob("q*.json")) or not pool.stored(suite):
        return
    if pool.load(suite) != snippets:
        message = f"the rebuilt pool differs from the one {suite.runs} was run on; move runs away"
        raise StageError(message)
