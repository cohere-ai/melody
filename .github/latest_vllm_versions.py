import json
import re
import urllib.request

from packaging.version import Version

DOCKER_TAGS_URL = (
    "https://hub.docker.com/v2/repositories/vllm/vllm-openai-cpu/tags?page_size=100"
)
VERSION_TAG = re.compile(r"^v(?P<version>\d+\.\d+\.\d+)$")
VERSION_COUNT = 3


def latest_vllm_versions() -> list[str]:
    versions = []
    url = DOCKER_TAGS_URL
    while url is not None:
        with urllib.request.urlopen(url, timeout=10) as response:
            data = json.load(response)

        for tag in data["results"]:
            match = VERSION_TAG.match(tag["name"])
            if match is None:
                continue
            if not any(image.get("architecture") == "amd64" for image in tag["images"]):
                continue
            version = Version(match.group("version"))
            if version.is_prerelease or version.is_devrelease:
                continue
            versions.append(version)

        url = data["next"]

    versions = sorted(set(versions), reverse=True)
    if len(versions) < VERSION_COUNT:
        raise RuntimeError(f"Only found {len(versions)} vLLM CPU Docker versions")

    return [str(version) for version in versions[:VERSION_COUNT]]


if __name__ == "__main__":
    print(json.dumps(latest_vllm_versions()))
