//
//  ContentPickerView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 23/2/25.
//

import Foundation
import SwiftUI
import PhotosUI

struct ContentPickerView: View {
    @State private var isShowingConfirmationDialog = false
    @State private var isShowingPhotosPicker = false
    
    @EnvironmentObject private var core: Core
    
    var body: some View {
        HStack {
            Text("Add new file")
                .modifier(Body2())
                .opacity(0.5)
            Button(action: {
                self.isShowingConfirmationDialog = true
            }) {
                Image(systemName: "plus")
                    .resizable()
                    .foregroundColor(Theme.LightViolet.color)
                    .padding(.all, 8)
                    .frame(width: 30, height: 30)
                    .background(
                        RoundedRectangle(cornerRadius: .infinity)
                            .fill(Theme.LightViolet.color.opacity(0.1))
                            .stroke(Theme.PrimaryViolet.color.opacity(0.45), lineWidth: 1)
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
            core.onMediasChanged()
        }
    }
}

#Preview {
    ContentPickerView()
}
