
switch (document.readyState) {
  case "loading":
    // The document is still loading.
	document.addEventListener('DOMContentLoaded', registerCallbacks);
    break;
  case "interactive":
    // The document has finished loading. We can now access the DOM elements.
    // But sub-resources such as scripts, images, stylesheets and frames are still loading.
	registerCallbacks();
    break;
  case "complete":
    // The page is fully loaded.
	registerCallbacks();
    break;
}


function registerCallbacks() {
	const uploadBtn = document.getElementById("file-upload-btn");
	uploadBtn.addEventListener('click', uploadFile);
}


function uploadFile() {
	const fileInput = document.getElementById("upload-file");
	const files = fileInput.files;
	const dirId = document.getElementById("dirid-field").value;
	
	for (var i = 0; i < files.length; i++) {
		const file = files.item(i);
		if (file.size < 65536) {
			const pushReq = new XMLHttpRequest();
			pushReq.addEventListener("load", onPushFile);
			pushReq.open("POST", "/upload/" + dirId + "/" + encodeURIComponent(file.name));
			pushReq.setRequestHeader("Content-Type", file.type);
			pushReq.send(file);
		} else {
			// TODO: Show error.
		}
	}
}

function onPushFile() {
	if (this.status == 200) {
		// Create new file list item:
		let newLi = document.createElement("li");
		console.log(this.responseText);
		newLi.innerHTML = this.responseText;

		// Append new list item:
		const fileList = document.getElementById("file-list");
		fileList.appendChild(newLi);
	} else {
		// TODO: Show error
	}
}
