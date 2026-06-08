param(
  [string]$Tag = "dev"
)

$Registry = $env:RHELMA_DOCKER_REGISTRY
function Img($Name, $Dockerfile) {
  $Target = "${Name}:${Tag}"
  if ($Registry -and $Registry.Trim() -ne "") { $Target = "$Registry/$Target" }
  Write-Host "==> Building $Target"
  docker build -f $Dockerfile -t $Target .
}

Img "node-registry" "deploy/rhelma6/docker/Dockerfile.node-registry"
Img "bridge-adapter" "deploy/rhelma6/docker/Dockerfile.bridge-adapter"
Img "rhelma-node" "deploy/rhelma6/docker/Dockerfile.rhelma-node"

Write-Host "Done."
