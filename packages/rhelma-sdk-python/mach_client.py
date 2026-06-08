python
import requests

class MachClient:
    def __init__(self, gateway_url, api_key):
        self.gateway_url = gateway_url
        self.api_key = api_key

    def get_node_status(self, node_id):
        response = requests.get(f"{self.gateway_url}/nodes/{node_id}/status", headers={
            "Authorization": f"Bearer {self.api_key}"
        })
        return response.json()
