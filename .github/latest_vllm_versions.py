import json
import urllib.request

from packaging.version import Version

PYPI_VLLM_URL = "https://pypi.org/pypi/vllm/json"
VERSION_COUNT = 3


def latest_vllm_versions() -> list[str]:
    with urllib.request.urlopen(PYPI_VLLM_URL, timeout=10) as response:
        data = json.load(response)

    versions = []
    for raw_version, files in data["releases"].items():
        version = Version(raw_version)
        if version.is_prerelease or version.is_devrelease:
            continue
        if not any(not file.get("yanked", False) for file in files):
            continue
        versions.append(version)

    return [str(version) for version in sorted(versions, reverse=True)[:VERSION_COUNT]]


if __name__ == "__main__":
    print(json.dumps(latest_vllm_versions()))
