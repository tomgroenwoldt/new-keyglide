variable "hcloud_token" {
    type = string
}

# Configure the Hetzner Cloud Provider
provider "hcloud" {
    token = "${var.hcloud_token}"
}

# Create a server
resource "hcloud_server" "web" {
  name        = "keyglide"
  image       = "ubuntu-20.04"
  server_type = "cx22"
  ssh_keys = [hcloud_ssh_key.default.id]
  user_data = file("user-data.yml")
  firewall_ids = [hcloud_firewall.myfirewall.id]
}

resource "hcloud_ssh_key" "default" {
  name       = "hetzner_key"
  public_key = file("../secrets/review_ssh_key.pub")
}

resource "hcloud_firewall" "myfirewall" {
  name = "my-firewall"
  rule {
    direction = "in"
    protocol  = "icmp"
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "3000"
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

}
