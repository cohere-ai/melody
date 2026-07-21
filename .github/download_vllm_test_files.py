import pathlib
import sys
import urllib.error
import urllib.request

TEST_PATHS = ("tests/reasoning/test_cohere_command_reasoning_parser.py",)
TARGET_DIR = pathlib.Path("vllm_upstream_tests")


def download_reasoning_tests(version: str) -> list[pathlib.Path]:
    downloaded = []
    for test_path in TEST_PATHS:
        url = f"https://raw.githubusercontent.com/vllm-project/vllm/v{version}/{test_path}"
        target = TARGET_DIR / test_path
        target.parent.mkdir(parents=True, exist_ok=True)

        try:
            with urllib.request.urlopen(url, timeout=10) as response:
                target.write_bytes(response.read())
        except urllib.error.HTTPError as error:
            if error.code == 404:
                print(f"Skipping: {url} does not exist", file=sys.stderr)
                continue
            raise

        downloaded.append(target)

    return downloaded


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: download_vllm_test_files.py <vllm-version>")

    for test_file in download_reasoning_tests(sys.argv[1]):
        print(test_file)
