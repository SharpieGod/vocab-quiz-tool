# Saves every .docx attachment from the tutor into ./documents/chinese
# pip install google-api-python-client google-auth-oauthlib ; needs credentials.json (Desktop OAuth client) next to it
import base64, os
from google_auth_oauthlib.flow import InstalledAppFlow
from googleapiclient.discovery import build

creds = InstalledAppFlow.from_client_secrets_file(
    "credentials.json", ["https://www.googleapis.com/auth/gmail.readonly"]).run_local_server(port=0)
gm = build("gmail", "v1", credentials=creds).users()
OUT = "documents/chinese"
os.makedirs(OUT, exist_ok=True)

q, token, saved = "from:caiwenyin1991@126.com has:attachment filename:docx", None, 0
while True:
    page = gm.messages().list(userId="me", q=q, pageToken=token).execute(num_retries=8)
    for m in page.get("messages", []):
        msg = gm.messages().get(userId="me", id=m["id"]).execute(num_retries=8)
        parts = msg["payload"].get("parts", [])
        while parts:
            p = parts.pop()
            parts += p.get("parts", [])
            name = os.path.basename(p.get("filename", ""))  # basename: never trust sender's path
            if not name.lower().endswith(".docx"):
                continue
            path = os.path.join(OUT, name)
            done = os.path.join(OUT, "processed_documents", name)  # helper.py moves files here
            if any(os.path.exists(f) and os.path.getsize(f) == p["body"].get("size") for f in (path, done)):
                continue  # already saved by an earlier run
            if os.path.exists(path):  # same name from another email: keep both
                path = os.path.join(OUT, f"{m['id']}_{name}")
            data = gm.messages().attachments().get(
                userId="me", messageId=m["id"], id=p["body"]["attachmentId"]).execute(num_retries=8)["data"]
            with open(path, "wb") as f:
                f.write(base64.urlsafe_b64decode(data))
            saved += 1
    token = page.get("nextPageToken")
    if not token:
        break
print(f"saved {saved} files to {OUT}")
