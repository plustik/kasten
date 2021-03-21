
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
			let header = new Headers();
			header.set("Content-Type", file.type);
			header.set("Accept", "text/json");
			fetch("/upload/" + dirId + "/" + encodeURIComponent(file.name),
				{
					method: "POST",
					headers: header,
					body: file,
				})
				.then(function(res) {
					if (res.status == 200) {
						return res.json()
					} else {
						// TODO: Show error
					}
				})
				.then(function(jsonRes) {
					onPushFile(jsonRes);
				});
		} else {
			// TODO: Show error.
		}
	}
}

function onPushFile(req) {
	// Create new file list item:
	let newLi = document.createElement("li");
	newLi.innerHTML = '<a href="/files/' + req.id + '">' + req.name + '</a>';

	// Append new list item:
	const fileList = document.getElementById("file-list");
	fileList.appendChild(newLi);
}
