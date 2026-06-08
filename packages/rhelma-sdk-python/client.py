python
import httpx
from dataclasses import dataclass

@dataclass
class MachConfig:
    gateway_url: str
    api_key: str
    timeout: float = 30.0

class MachClient:
    def __init__(self, config: MachConfig):
        self._config = config
        self._http = httpx.AsyncClient(
            base_url=config.gateway_url,
            headers={"Authorization": f"Bearer {config.api_key}"},
            timeout=config.timeout,
        )

    async def get_balance(self, subject_id: str) -> int:
        response = await self._http.get(f"/v1/credits/{subject_id}")
        response.raise_for_status()
        return response.json()["balance"]
    
    async def register_node(self, display_name: str, **kwargs) -> dict:
        response = await self._http.post("/v1/nodes/register", json={
            "display_name": display_name,
            **kwargs
        })
        response.raise_for_status()
        return response.json()
