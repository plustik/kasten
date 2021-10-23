
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
	// DEBUG
	//showNewDir();
	// Mkdir push button:
	const action_list = document.getElementById("action_list");
	// Add seperator:
	let seperator = document.createElement("span");
	seperator.setAttribute("class", "barsep");
	seperator.innerHTML = '&#160;|&#160;';
	action_list.appendChild(seperator);
	// Add button:
	let mkDirSpan = document.createElement("span");
	mkDirSpan.setAttribute("class", "tab");
	let mkDirBtn = document.createElement("button");
	mkDirBtn.setAttribute("type", "button");
	mkDirBtn.setAttribute("class", "action-button");
	mkDirBtn.innerHTML = "mkdir";
	mkDirBtn.addEventListener('click', showNewDir);
	mkDirSpan.appendChild(mkDirBtn);
	action_list.appendChild(mkDirSpan);

	return;

	// Upload push button:
	const uploadBtn = document.getElementById("file-upload-btn");
	uploadBtn.addEventListener('click', uploadFile);

	// Edit forms:

	const showUploadFormBtn = document.getElementById("show-upload-btn");
	showUploadFormBtn.addEventListener('click', showUploadForm);

	const hideUploadFormBtn = document.getElementById("hide-upload-form-btn");
	hideUploadFormBtn.addEventListener('click', hideUploadForm);

	const showMkdirFormBtn = document.getElementById("show-mkdir-btn");
	showMkdirFormBtn.addEventListener('click', showMkdirForm);

	const hideMkdirFormBtn = document.getElementById("hide-mkdir-form-btn");
	hideMkdirFormBtn.addEventListener('click', hideMkdirForm);

	// Dir action buttons:

	var dirActionMenus = document.getElementsByClassName("dir-action-menu");
	var dirActionBtns = [];
	var dirActionDrops = [];
	for (let i=0; i<dirActionMenus.length; i++) {
		// Find button and dropdown:
		for (let j=0; j < dirActionMenus[i].childNodes.length; j++) {
			if (dirActionMenus[i].childNodes[j].className === "dir-action-btn") {
				dirActionBtns[i] = dirActionMenus[i].childNodes[j];
			} else if (dirActionMenus[i].childNodes[j].className === "dir-action-drop") {
				dirActionDrops[i] = dirActionMenus[i].childNodes[j];
			}

		}

		// Toggle dir action dropdown:
		var index = i;
		dirActionBtns[index].addEventListener("click", () => {

			if (dirActionDrops[index].style.display === "none") {;
				dirActionDrops[index].style.display = "block";
			} else {
				dirActionDrops[index].style.display = "none";
			}
		});

		// Remove directory function:
		// Find rm button:
		const actionElements = dirActionDrops[i].childNodes;
		for (let j=0; j < actionElements.length; j++) {
			if (actionElements[j].className === "drop-item" && actionElements[j].textContent === "Remove") {
				actionElements[j].addEventListener("click", () => {
					console.log(index);
					console.log(dirActionMenus);
					removeDirLi(dirActionMenus[index].parentElement);
				});
			}
		}
	}

	// File action menu/buttons:

	var menus = document.getElementsByClassName("file-action-menu");
	for (let i=0; i<menus.length; i++) {
		// Find button and dropdown:
		for (let j=0; j < menus[i].childNodes.length; j++) {
			if (menus[i].childNodes[j].className === "file-action-btn") {
				var btn = menus[i].childNodes[j];
			} else if (menus[i].childNodes[j].className === "file-action-drop") {
				var dropdown = menus[i].childNodes[j];
			}

		}

		// Toggle file action dropdown:
		btn.addEventListener("click", () => {
			if (dropdown.style.display === "none") {;
				dropdown.style.display = "block";
			} else {
				dropdown.style.display = "none";
			}
		});

		// Remove file function:
		// Find rm button:
		const actionElements = dropdown.childNodes;
		for (let j=0; j < actionElements.length; j++) {
			if (actionElements[j].className === "drop-item" && actionElements[j].textContent === "Remove") {
				actionElements[j].addEventListener("click", () => {
					removeFileLi(menus[i].parentElement);
				});
			}
		}
	}
}


//
// General functions
//
//
function verifyName(name) {
	if (name.length === 0) {
		return "The given name is to short.";
	}

	return true;
}

function serializeBigInt(key, value) {
	if (typeof value === "bigint") {
		return value.toString();
	} else {
		return value;
	}
}


//
// Upload files
//

function showUploadForm() {
	const addDropdown = document.getElementById("dir-add-drop");
	if (addDropdown.style.display === "block") {;
		addDropdown.style.display = "none";
	}

	const uploadForm = document.getElementById("upload-form");
	uploadForm.style.display = "block";
}
function hideUploadForm() {
	const uploadForm = document.getElementById("upload-form");
	uploadForm.style.display = "none";
}


function uploadFile() {
	const fileInput = document.getElementById("upload-file");
	const files = fileInput.files;
	const dirId = document.getElementById("current-dir-li").getAttribute("dir_id");
	
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
	newLi.innerHTML = '<a class="file-name" href="/files/' + Number(req.id).toString(16) + '" download="' + req.name + '">' + req.name + '</a>';
	newLi.setAttribute("class", "file-item");

	// Append new list item:
	const fileList = document.getElementById("file-list");
	fileList.appendChild(newLi);

	// Hide upload form:
	hideUploadForm();
}

//
// Adding directories
//

function showNewDir() {
	let contentList = document.getElementById("content-list");
	const lastRowClass = contentList.rows.item(contentList.rows.length - 1).getAttribute("class");

	let newRow = contentList.insertRow(-1);
	if (lastRowClass === "light") {
		newRow.setAttribute("class", "dark");
	} else if (lastRowClass === "dark") {
		newRow.setAttribute("class", "light");
	} else {
		console.log("Error: Unkown class of last row.");
		return;
	}

	let modeField = newRow.insertCell(-1);
	modeField.setAttribute("class", "mode");
	modeField.innerHTML = "drw";

	let sizeField = newRow.insertCell(-1);
	sizeField.setAttribute("class", "size");
	sizeField.innerHTML = "&#160;";

	let nameField = newRow.insertCell(-1);
	nameField.setAttribute("class", "list");
	nameField.innerHTML = '<input type="text" class="new-name-input" id="new-name" autocomplete="off" placeholder="NAME">';
	newRow.appendChild(nameField);

	let addDirBtn = document.createElement("button");
	addDirBtn.setAttribute("type", "button");
	addDirBtn.setAttribute("class", "link-button");
	addDirBtn.innerHTML = "add";
	addDirBtn.addEventListener('click', pushDir);
	let linkField = newRow.insertCell(-1);
	linkField.setAttribute("class", "link");
	linkField.appendChild(addDirBtn);

	document.getElementById("new-name").focus();
}

function pushDir() {
	const dirName = document.getElementById("new-name").value;
	if (verifyName(dirName) != true) {
		document.getElementById("new-name").style.backgroundColor = '#ff9999';
		return;
	}
	const parentId = document.getElementById("current-dir-id").getAttribute("dir_id");

	let header = new Headers();
	header.set("Accept", "text/json");

	let reqData = {};
	reqData.parent_id = BigInt("0x" + parentId);
	console.log(parentId);
	console.log(reqData.parent_id);
	console.log(reqData);
	reqData.name = dirName;

	fetch("/rest_api/dirs/",
		{
			method: "POST",
			headers: header,
			body: JSON.stringify(reqData, serializeBigInt),
			mode: "same-origin",
			redirect: "error",
		})
		.then(function(res) {
			if (res.status == 200) {
				return res.json()
			} else {
				// TODO: Show error
			}
		})
		.then(function(jsonRes) {
			onPushDir(jsonRes);
		});

	let contentList = document.getElementById("content-list");
	contentList.deleteRow(-1);
}

function onPushDir(req) {
	// Create new dir list item:
	let newLi = document.createElement("li");
	newLi.innerHTML = req.name;
	newLi.innerHTML = '<a class="dir-name" href="/dirs/' + Number(req.id).toString(16) + '/view.html">' + req.name + '/</a>';
	newLi.setAttribute("class", "dir-item");

	// Append new list item:
	const fileList = document.getElementById("file-list");
	fileList.appendChild(newLi);

	let contentList = document.getElementById("content-list");
	const lastRowClass = contentList.rows.item(contentList.rows.length - 1).getAttribute("class");

	let newRow = contentList.insertRow(-1);
	if (lastRowClass === "light") {
		newRow.setAttribute("class", "dark");
	} else if (lastRowClass === "dark") {
		newRow.setAttribute("class", "light");
	} else {
		console.log("Error: Unkown class of last row.");
		return;
	}

	let modeField = newRow.insertCell(-1);
	modeField.setAttribute("class", "mode");
	modeField.innerHTML = "drw";

	let sizeField = newRow.insertCell(-1);
	sizeField.setAttribute("class", "size");
	sizeField.innerHTML = "&#160;";

	let nameField = newRow.insertCell(-1);
	nameField.setAttribute("class", "list");
	nameField.innerHTML = '<a href="dirs/' + Number(req.id).toString(16) + '/view.html">' + req.name + '</a>';
	newRow.appendChild(nameField);

	let linkField = newRow.insertCell(-1);
	linkField.setAttribute("class", "link");
	linkField.innerHTML = '<a href="/dirs/' + Number(req.id).toString(16)
		+ '/zip">download</a><span class="barsep">&#160;|&#160;</span><a class="showlink" href="dirs/'
		+ Number(req.id).toString(16) + '/view.html">show</a>'
}

//
// Action buttons:
//
function removeDirLi(li) {
	for (let i=0; i<li.childNodes.length; i++) {
		if (li.childNodes[i].className === "dir-name") {
			const dirId = li.childNodes[i].getAttribute("dir_id");

			let header = new Headers();
			header.set("Accept", "text/json");

			fetch("/dirs/" + dirId,
				{
					method: "DELETE",
					headers: header,
				})
				.then(function(res) {
					if (res.status == 200) {
						li.parentElement.removeChild(li);
						return res.json()
					} else {
						// TODO: Show error
					}
				});

			break;
		}
	}
}

function removeFileLi(li) {
	for (let i=0; i<li.childNodes.length; i++) {
		if (li.childNodes[i].className === "file-name") {
			const dirId = li.childNodes[i].getAttribute("file_id");

			let header = new Headers();
			header.set("Accept", "text/json");

			fetch("/files/" + dirId,
				{
					method: "DELETE",
					headers: header,
				})
				.then(function(res) {
					if (res.status == 200) {
						li.parentElement.removeChild(li);
						return res.json()
					} else {
						// TODO: Show error
					}
				});

			break;
		}
	}
}
