//
//  AddContentButton.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 22/2/25.
//

import Foundation
import SwiftUI

enum PickerType: Identifiable {
    case photo, file
    
    var id: Int {
        hashValue
    }
}

struct AddContentButton: View {
    @State private var isShowingConfirmationDialog = false
    @State private var selectedType: PickerType?
    
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
                        self.selectedType = .photo
                    }
                    Button("Files") {
                        self.selectedType = .file
                    }
                }
        }
        .sheet(item: self.$selectedType, onDismiss: { self.selectedType = nil }) { item in
            switch item {
            case .file: NavigationView {
                FilePickerView()
            }
            case .photo: NavigationView {
                MediaPickerView()
            }
            }
        }
    }
}

#Preview {
    AddContentButton()
}
