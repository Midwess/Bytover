//
//  ContentPickerView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 23/2/25.
//

import Foundation
import SwiftUI
import PhotosUI
import SharedTypes

struct ContentPickerView: View {
    @State private var isShowingConfirmationDialog = false
    @State private var isShowingPhotosPicker = false
    @State private var isShowingFilePicker = false
    @State private var selectedResources: [SelectedResourceViewModel] = []

    @EnvironmentObject private var core: Core

    var body: some View {
        VStack {
            Button(action: {
                self.isShowingConfirmationDialog = true
            }) {
                HStack {
                    if self.selectedResources.isEmpty {
                        Text("Add files now")
                            .modifier(Label2())
                            .foregroundStyle(Theme.PrimaryText.color.opacity(0.7))
                            .padding(.trailing, 1)
                    } else {
                        Spacer()
                    }

                    Image(systemName: "plus")
                        .resizable()
                        .foregroundColor(Theme.PrimaryText.color)
                        .fontWeight(.bold)
                        .padding(.all, 8)
                        .frame(width: 30, height: 30)
                        .background(
                            RoundedRectangle(cornerRadius: .infinity)
                                .fill(Theme.BluePrimary.color.opacity(1))
                        )
                        .confirmationDialog(
                            "Hey! What type of file do you want to add?",
                            isPresented: self.$isShowingConfirmationDialog) {
                                Button("Photos and videos") {
                                    self.isShowingPhotosPicker = true
                                }
                                Button("Files") {
                                    self.isShowingFilePicker = true
                                }
                                Button(action: {}) {
                                    Text("Can not find what you're looking for? 🤷‍♂️")
                                }
                            }
                }
            }
            .fileImporter(isPresented: $isShowingFilePicker, allowedContentTypes: [.item, .folder, .directory, .compositeContent, .content],
                allowsMultipleSelection: true,
                onCompletion: {
                result in
                switch result {
                case .success(let urls):
                    Task {
                        await core.onFileSelected(urls: urls)
                    }
                case .failure(let error):
                    core.toastMessage.value = "Failed to select files"
                }
            })
            .photosPicker(isPresented: $isShowingPhotosPicker,
                          selection: $core.selectedMediaItems,
                          selectionBehavior: .ordered,
                          matching: .any(of: [.images, .videos]),
                          preferredItemEncoding: .automatic,
                          photoLibrary: .shared()
            )
            .onChange(of: core.selectedMediaItems) { _, _ in
                Task {
                    await core.onMediasChanged()
                }
            }
            .padding(.horizontal, SpaceTheme.screen.value)
            .foregroundStyle(Theme.PrimaryText.color)
            .onReceive(self.core.transfer, perform: { value in
                if value?.selected_resources.count != self.selectedResources.count {
                    self.selectedResources = value?.selected_resources ?? []
                }
            })
        }
    }
}

#Preview {
    ContentPickerView()
        .environmentObject(CoreMock())
}
