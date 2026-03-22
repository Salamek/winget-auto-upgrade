import enum
import os
import sys
from pathlib import Path
from types import TracebackType
from typing import Self
from urllib.parse import urlparse

import requests


@enum.unique
class FileArchitectureEnum(enum.Enum):
    X_64 = 'x64'
    X_86 = 'x86'


@enum.unique
class FileTypeEnum(enum.Enum):
    EXE = 'EXE'
    MSI = 'MSI'
    MSIX = 'MSIX'
    APPX = 'APPX'
    ZIP = 'ZIP'
    INNO = 'INNO'
    NULLSOFT = 'NULLSOFT'
    WIX = 'WIX'
    BURN = 'BURN'

@enum.unique
class FileScopeEnum(enum.Enum):
    MACHINE = 'machine'
    USER = 'user'


class WingetRepoAuthorized:
    base_path: str
    auth_token: str
    def __init__(self, base_path: str, auth_token: str) -> None:
        self.base_path = base_path
        self.auth_token = auth_token

    def test(self) -> str:
        response = requests.get(
            f'{self.base_path}/test',
            headers={
                'Authorization': f'Bearer {self.auth_token}',
            },
            timeout=30,
        )

        response.raise_for_status()

        message = response.json().get('Message')
        if not message:
            msg = 'message was not found in response'
            raise ValueError(msg)

        return message

    def add_package_version(
        self,
        package_id: str,
        file: Path,
        package_version: str,
        file_architecture: FileArchitectureEnum,
        file_type: FileTypeEnum,
        file_scope: FileScopeEnum,
    ) -> str:

        response = requests.post(
            f'{self.base_path}/add_package_version/{package_id}',
            data={
                'package_version': package_version,
                'file_architect': file_architecture.value,
                'file_type': file_type.value,
                'file_scope': file_scope.value,
            },
            files = {
                'file': file.open('rb'),
            },
            headers={
                'Authorization': f'Bearer {self.auth_token}',
            },
            timeout=30,
        )

        response.raise_for_status()

        uid = response.json().get('UID')
        if not uid:
            msg = 'UID was not found in response'
            raise ValueError(msg)

        return uid


    def logout(self) -> None:
        response = requests.post(
            f'{self.base_path}/logout',
            data={
                'token': self.auth_token,
            },
            headers={
                'Authorization': f'Bearer {self.auth_token}',
            },
            timeout=30,
        )

        response.raise_for_status()

    def __enter__(self) -> Self:
        return self

    def __exit__(self, exc_type: type[BaseException] | None, exc_value: BaseException | None, traceback: TracebackType | None) -> None:
        self.logout()


class WingetRepo:
    base_path: str
    username: str
    password: str

    def __init__(self, uri: str) -> None:
        parsed = urlparse(uri)
        self.base_path = f'{parsed.scheme}://{parsed.hostname}{parsed.path}'
        if not parsed.username:
            msg = 'username is not provided'
            raise ValueError(msg)

        if not parsed.password:
            msg = 'password is not provided'
            raise ValueError(msg)
        self.username = parsed.username
        self.password = parsed.password

    def get_auth_token(self, username: str | None, password: str | None) -> str:
        response = requests.post(f'{self.base_path}/login', data={
            'username': username or self.username,
            'password': password or self.password,
        }, timeout=10)

        response.raise_for_status()

        token = response.json().get('message')
        if not token:
            msg = 'Response does not contain token'
            raise ValueError(msg)

        return token

    def login(self, username: str | None = None, password: str | None = None) -> WingetRepoAuthorized:
        auth_token = self.get_auth_token(username=username, password=password)
        return WingetRepoAuthorized(self.base_path, auth_token)


if __name__ == '__main__':
    version = os.getenv('CI_COMMIT_TAG')
    if not version:
        print('CI_COMMIT_TAG not provided in env')  # noqa: T201
        sys.exit(1)

    winget_credentials = os.getenv('WINGET_CREDENTIALS')
    if not winget_credentials:
        print('WINGET_CREDENTIALS not provided in env')  # noqa: T201
        sys.exit(1)

    winget_repo = WingetRepo(f'https://{winget_credentials}@repository.salamek.cz/win/client/api')

    with winget_repo.login() as session:
        session.add_package_version(
            'salamek.winget-auto-upgrade',
            Path('windows/winget-auto-upgrade.msi'),
            version,
            FileArchitectureEnum.X_64,
            FileTypeEnum.MSI,
            FileScopeEnum.MACHINE,
        )
