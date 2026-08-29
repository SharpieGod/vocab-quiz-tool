import os
import docx2txt

directory = "./documents/chinese"
processed = "./documents/chinese/processed_documents"
out = "./documents/chinese/txt"

for filename in os.listdir(directory):
    if filename.endswith(".docx"):
        docx_path = os.path.join(directory, filename)
        processed_path = os.path.join(processed, filename)
        txt_path = os.path.join(out, filename.replace(".docx", ".txt"))

        text_content = docx2txt.process(docx_path)


        with open(txt_path, "w", encoding="utf-8") as txt_file:
            txt_file.write(text_content)

        os.rename(docx_path, processed_path)
