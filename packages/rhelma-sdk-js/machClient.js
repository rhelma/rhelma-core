javascript
class MachClient {
    constructor(gatewayUrl, apiKey) {
        this.gatewayUrl = gatewayUrl;
        this.apiKey = apiKey;
    }

    async getBalance(subjectId) {
        const response = await fetch(`${this.gatewayUrl}/v1/credits/${subjectId}`, {
            method: 'GET',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`
            }
        });
        return response.json().then(data => data.balance);
    }

    async registerNode(displayName, params) {
        const response = await fetch(`${this.gatewayUrl}/v1/nodes/register`, {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                display_name: displayName,
                ...params
            })
        });
        return response.json();
    }
}

export default MachClient;
