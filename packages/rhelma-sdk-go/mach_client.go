go
package machclient

import (
    "encoding/json"
    "fmt"
    "net/http"
)

type MachClient struct {
    GatewayUrl string
    ApiKey     string
}

func (client *MachClient) GetBalance(subjectId string) (int, error) {
    url := fmt.Sprintf("%s/v1/credits/%s", client.GatewayUrl, subjectId)
    req, err := http.NewRequest("GET", url, nil)
    if err != nil {
        return 0, err
    }
    req.Header.Set("Authorization", "Bearer "+client.ApiKey)

    resp, err := http.DefaultClient.Do(req)
    if err != nil {
        return 0, err
    }
    defer resp.Body.Close()

    var result map[string]interface{}
    if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
        return 0, err
    }
    return int(result["balance"].(float64)), nil
}

func (client *MachClient) RegisterNode(displayName string, params map[string]interface{}) (map[string]interface{}, error) {
    url := fmt.Sprintf("%s/v1/nodes/register", client.GatewayUrl)
    jsonData, _ := json.Marshal(map[string]interface{}{
        "display_name": displayName,
        "params":       params,
    })
    req, err := http.NewRequest("POST", url, json.NewDecoder(jsonData))
    if err != nil {
        return nil, err
    }
    req.Header.Set("Authorization", "Bearer "+client.ApiKey)
    req.Header.Set("Content-Type", "application/json")

    resp, err := http.DefaultClient.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var result map[string]interface{}
    if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
        return nil, err
    }
    return result, nil
}
