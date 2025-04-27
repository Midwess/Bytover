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
                    }
                    else {
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
                }
                .confirmationDialog(
                    "Hey! What type of file do you want to add?",
                    isPresented: self.$isShowingConfirmationDialog) {
                        Button("Photos and videos") {
                            self.isShowingPhotosPicker = true
                        }
                        Button("Files") {
                            
                        }
                        Button(action: {}) {
                            Text("Can not find what you're looking for? 🤷‍♂️")
                        }
                    }
            }
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
            .padding(.horizontal, 16)
            .padding(.vertical, 8)
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
